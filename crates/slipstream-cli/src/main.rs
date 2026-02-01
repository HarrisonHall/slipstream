//! Slipstream.

mod cli;
mod config;
mod database;
mod feeds;
mod logging;
mod modes;
pub mod prelude;

#[cfg(test)]
mod tests;

use prelude::internal::*;
use prelude::*;

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
    color_eyre::install()?;
    let cli = Cli::parse();

    // If doing the config mode, we don't want to go any further.
    match &cli.command {
        CommandMode::Config { config_mode } => {
            let config_path = match cli.config_path() {
                Ok(cp) => cp,
                Err(e) => bail!("Failed to determine config path: {e}"),
            };
            return config_cli(config_mode.clone(), config_path);
        }
        _ => {}
    };

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
    match &cli.command {
        CommandMode::Serve { port } => tasks.spawn(serve_cli(
            port.clone(),
            config.clone(),
            updater_handle,
            cancel_token.clone(),
        )),
        CommandMode::Read => tasks.spawn(read_cli(
            config.clone(),
            updater_handle,
            cancel_token.clone(),
        )),
        CommandMode::Config { .. } => unreachable!(),
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
