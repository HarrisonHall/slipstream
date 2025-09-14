//! CLI.

use super::*;

/// Slipstream cli parsing.
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Configuration file.
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,
    /// Launch in debug. This lowers in the log level to debug. In read mode, this
    /// additionally logs to the cli, which is disabled by default.
    #[arg(short, long, action)]
    pub debug: bool,
    /// Display logs and log at the trace level.
    #[arg(short, long, action)]
    pub verbose: bool,
    /// The command mode for slipstream.
    #[command(subcommand)]
    pub command: CommandMode,
}

impl Cli {
    /// Parse configuration.
    pub fn parse_config(&self) -> Result<Config> {
        // Get specified config path.
        let config_path: PathBuf = match &self.config {
            Some(path) => path.clone(),
            None => match PathBuf::from_str(&*DEFAULT_CONFIG_DIR) {
                Ok(p) => p,
                Err(e) => {
                    bail!(
                        "Invalid default config {}: {}.",
                        &*DEFAULT_CONFIG_DIR,
                        e
                    );
                }
            },
        }
        .resolve()
        .into();
        // Make directory if it doesn't exist.
        if let Some(parent_dir) = config_path.parent() {
            if !parent_dir.exists() {
                std::fs::create_dir_all(parent_dir).ok();
            }
        }
        // Make file if it doesn't exist.
        if !config_path.exists() {
            tracing::debug!("Created config file @ {:?}.", config_path);
            if let Err(e) = std::fs::File::create(config_path.as_path()) {
                bail!(
                    "Unable to create config file at {:?}: {}.",
                    config_path,
                    e
                );
            }
        }
        // Read file.
        let config_data = match std::fs::read_to_string(&config_path) {
            Ok(data) => data,
            Err(e) => {
                bail!(
                    "Unable to read data from config file {:?}: {}.",
                    config_path,
                    e
                );
            }
        };
        // Parse.
        match toml::from_str(&config_data) {
            Ok(config) => Ok(config),
            Err(e) => {
                bail!("Configuration file is not valid: {}.", e);
            }
        }
    }
}

/// Slipstream command mode.
#[derive(Subcommand)]
pub enum CommandMode {
    /// Serve feeds as static webpages and atom exports.
    Serve {
        /// TODO
        #[arg(short, long, value_name = "PORT")]
        port: Option<u16>,
    },
    /// Read feeds in a local tui.
    Read {},
}
