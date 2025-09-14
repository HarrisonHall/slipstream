//! Slipstream.

use std::cell::LazyCell;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::{path::PathBuf, str::FromStr};

use anyhow::{Result, bail};
use atom_syndication::{self as atom};
use clap::{Parser, Subcommand};
use resolve_path::PathResolveExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use slipstream_feeds::{self as slipfeed};

mod cli;
mod config;
mod database;
mod feeds;
mod logging;
mod modes;

use cli::*;
use config::*;
use database::*;
use feeds::*;
use logging::*;
use modes::*;

const DEFAULT_CONFIG_DIR: LazyCell<String> = LazyCell::new(|| {
    use directories::ProjectDirs;
    if let Some(dirs) = ProjectDirs::from("", "", "slipstream") {
        let mut config = dirs.config_dir().to_path_buf();
        config.push("slipstream.toml");
        String::from(config.to_string_lossy())
    } else {
        "~/.config/slipstream/slipstream.toml".to_owned()
    }
});
const DEFAULT_PORT: u16 = 3000;
const DEFAULT_UPDATE_SEC: u16 = 120;

/// Entry point for slipstream.
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<()> {
    // Initial setup.
    let cli = Cli::parse();
    let config = Arc::new(match cli.parse_config() {
        Ok(config) => config,
        Err(e) => bail!("Failed to parse config:\n{e}"),
    });
    setup_logging(&cli, &config)?;

    let cancel_token = CancellationToken::new();
    let mut tasks = JoinSet::new();

    // Run feed updates:
    let mut updater = config.updater().await?;
    let updater_handle = updater.handle()?;
    tasks.spawn(update(updater, config.clone(), cancel_token.clone()));

    // Run the command:
    match cli.command {
        CommandMode::Serve { port } => tasks.spawn(serve(
            port,
            config.clone(),
            updater_handle,
            cancel_token.clone(),
        )),
        CommandMode::Read {} => tasks.spawn(read(
            config.clone(),
            updater_handle,
            cancel_token.clone(),
        )),
    };

    // Wait for ctrl+c (top-level):
    {
        let cancel_token = cancel_token.clone();
        tasks.spawn(async move {
            tokio::select! {
                _ = cancel_token.cancelled() => {},
                _ = tokio::signal::ctrl_c() => {
                    cancel_token.cancel();
                },
            };
            Ok(())
        });
    }

    // Wait for tasks to complete.
    while let Some(task_res) = tasks.join_next().await {
        // If the task failed, print the error.
        if let Err(e) = task_res {
            tracing::error!("{}", e);
        }

        // Kill all other tasks.
        cancel_token.cancel();
    }

    Ok(())
}
