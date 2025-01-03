//! Slipknot.

use std::collections::HashMap;
use std::sync::Arc;
use std::{path::PathBuf, str::FromStr};

use atom_syndication::{self as atom};
use chrono::Duration;
use clap::Parser;
use resolve_path::PathResolveExt;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

mod cli;
mod config;
mod feeds;
mod filters;
mod limits;
mod logging;
// mod tests;

use cli::*;
use config::*;
use feeds::*;
use filters::*;
use limits::*;
use logging::*;

const DEFAULT_CONFIG_DIR: &str = "~/.config/slipknot/slipknot.toml";
const DEFAULT_PORT: u16 = 3000;
const DEFAULT_UPDATE_SEC: u16 = 120;

#[derive(Debug)]
enum Error {
    InvalidConfig,
}

/// Entry point for slipknot.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    // Initial setup.
    let cli = Cli::parse();
    let config = Arc::new(cli.parse_config().expect("Unable to parse config."));
    setup_logging(&cli, &config);

    // Allow updates to run in the background.
    let updater = Arc::new(Mutex::new(config.updater()));
    {
        let updater = updater.clone();
        tokio::task::spawn(async move {
            loop {
                {
                    let mut guard = updater.lock().await;
                    guard.update().await;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        });
    }

    // Create server.
    let app = axum::Router::new()
        .route("/", axum::routing::get(get_all))
        .route("/all", axum::routing::get(get_all))
        .route("/feed/*feed", axum::routing::get(get_feed))
        .route("/tag/*tag", axum::routing::get(get_tag))
        .route("/config", axum::routing::get(get_config))
        .with_state(Arc::new(SFState {
            updater,
            config: config.clone(),
        }));
    let port = cli.port.unwrap_or(config.port.unwrap_or(DEFAULT_PORT));
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect(&format!("Unable to bind to port {}", port));

    // Serve.
    tracing::info!("Serving feeds @ 0.0.0.0:{}", port);
    axum::serve(listener, app).await.expect("Error serving.");

    Ok(())
}

use axum::extract::State;
#[derive(Clone)]
struct SFState {
    updater: Arc<Mutex<Updater>>,
    config: Arc<Config>,
}
type StateType = axum::extract::State<Arc<SFState>>;

async fn get_all(State(state): StateType) -> impl axum::response::IntoResponse {
    tracing::debug!("/all");
    let updater = state.updater.lock().await;
    return (
        [(axum::http::header::CONTENT_TYPE, "application/atom+xml")],
        updater.syndicate_all(&state.config),
    );
}

async fn get_feed(
    State(state): StateType,
    uri: axum::http::Uri,
) -> impl axum::response::IntoResponse {
    tracing::debug!("{}", uri.path());
    let feed = &uri.path()["/feed/".len()..];
    let updater = state.updater.lock().await;
    return (
        [(axum::http::header::CONTENT_TYPE, "application/atom+xml")],
        updater.syndicate_feed(feed, &state.config),
    );
}

async fn get_tag(
    State(state): StateType,
    uri: axum::http::Uri,
) -> impl axum::response::IntoResponse {
    tracing::debug!("{}", uri.path());
    let tag = &uri.path()["/tag/".len()..];
    let updater = state.updater.lock().await;
    return (
        [(axum::http::header::CONTENT_TYPE, "application/atom+xml")],
        updater.syndicate_tag(tag, &state.config),
    );
}

async fn get_config(
    State(state): StateType,
) -> impl axum::response::IntoResponse {
    tracing::debug!("/config");
    let mut with_feed = String::new();
    state
        .config
        .serialize(toml::Serializer::new(&mut with_feed))
        .ok();
    return (
        [(axum::http::header::CONTENT_TYPE, "application/toml")],
        with_feed,
    );
}
