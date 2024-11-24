//! Slipknot.

use std::collections::HashMap;
use std::sync::Arc;
use std::{path::PathBuf, str::FromStr};

use atom_syndication as atom;
use chrono::Duration;
use clap::Parser;
use resolve_path::PathResolveExt;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

mod cli;
mod config;
mod feeds;
mod tests;

use cli::Cli;
use config::Config;
use feeds::{Feed, Updater};

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
    // Parse cli.
    let cli = Cli::parse();
    // Parse config.
    let config = cli.parse_config().expect("Unable to parse config.");

    let mut with_feed = String::new();
    config.serialize(toml::Serializer::new(&mut with_feed)).ok();
    println!("with_feed\n{}", with_feed);

    // Allow updates to run in the background
    let updater = Arc::new(Mutex::new(config.updater()));
    {
        let updater = updater.clone();
        tokio::task::spawn(async move {
            loop {
                {
                    let mut guard = updater.lock().await;
                    // println!("Updating...");
                    guard.update().await;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        });
    }

    // Create server.
    let app = axum::Router::new()
        .route("/feed/*feed", axum::routing::get(get_feed))
        .route("/tag/*tag", axum::routing::get(get_tag))
        .with_state(updater);
    let port = cli.port.unwrap_or(config.port.unwrap_or(DEFAULT_PORT));
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect(&format!("Unable to bind to port {}", port));

    // Serve.
    println!("Serving feeds @ 0.0.0.0:{}", port);
    axum::serve(listener, app).await.expect("Error serving.");

    Ok(())
}

async fn get_feed(
    axum::extract::State(updater): axum::extract::State<Arc<Mutex<Updater>>>,
    uri: axum::http::Uri,
) -> impl axum::response::IntoResponse {
    let feed = &uri.path()["/feed/".len()..];
    let updater = updater.lock().await;
    return (
        [(axum::http::header::CONTENT_TYPE, "application/atom+xml")],
        updater.syndicate(feed),
    );
}

async fn get_tag(uri: axum::http::Uri) -> String {
    let tag = &uri.path()["/tag/".len()..];
    return format!("Hello tag: {tag}");
}
