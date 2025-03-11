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
use tokio::sync::{Mutex, RwLock};

mod cli;
mod config;
mod feeds;
mod logging;
mod modes;

use cli::*;
use config::*;
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
    let config = Arc::new(cli.parse_config().expect("Unable to parse config."));
    setup_logging(&cli, &config);

    // Allow updates to run in the background.
    let updater = Arc::new(Mutex::new(config.updater().await));
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

    match cli.command {
        Mode::Serve { port } => {
            serve(port, config.clone(), updater.clone()).await?;
        }
        Mode::Read {} => {
            read(config.clone(), updater.clone()).await?;
        }
    }

    Ok(())
}
