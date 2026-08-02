//! Slipstream.

mod cli;
mod config;
mod database;
mod feeds;
mod logging;
mod modes;
pub mod prelude;
mod task_manager;

#[cfg(test)]
mod tests;

use prelude::internal::*;
use prelude::*;

static DEFAULT_CONFIG_DIR: LazyLock<String> = LazyLock::new(|| {
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
const DEFAULT_ADDRESS: &str = "0.0.0.0";
const DEFAULT_UPDATE_SEC: u16 = 120;

/// Entry point for slipstream.
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<()> {
    // Initial setup.
    color_eyre::install()?;
    let cli = Cli::parse();

    // Handle basic tasks:
    match &cli.command {
        CommandMode::Config { config_mode } => {
            let config_path = match cli.config_path() {
                Ok(cp) => cp,
                Err(e) => bail!("Failed to determine config path: {e}"),
            };
            return config_cli(config_mode.clone(), config_path);
        }
        CommandMode::Fetch { url, feed, format } => {
            return modes::fetch_cli(
                cli.parse_config()?,
                url,
                feed,
                FetchOutputFormat::from(
                    format.clone().unwrap_or_default().as_str(),
                ),
            )
            .await;
        }
        _ => {}
    }

    let config = Arc::new(match cli.parse_config() {
        Ok(config) => config,
        Err(e) => bail!("Failed to parse config:\n{e}"),
    });
    setup_logging(&cli, &config)?;

    let cancel_token = CancellationToken::new();
    let mut tasks = JoinSet::new();

    // Handle tasks, including background updates.
    let task_manager = config.build_task_manager().await?;
    let task_manager_handle = task_manager.handle()?;
    tasks.spawn(manage_tasks(
        task_manager,
        config.clone(),
        cancel_token.clone(),
    ));

    // Handle long-running tasks:
    match &cli.command {
        CommandMode::Serve { port, address } => tasks.spawn(serve_cli(
            *port,
            address.clone(),
            config.clone(),
            task_manager_handle,
            cancel_token.clone(),
        )),
        CommandMode::Read => tasks.spawn(read_cli(
            config.clone(),
            task_manager_handle,
            cancel_token.clone(),
        )),
        _ => unreachable!(),
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
