//! CLI.

use super::*;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,
    #[arg(short, long, action)]
    pub debug: bool,
    #[command(subcommand)]
    pub command: Mode,
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

#[derive(Subcommand)]
pub enum Mode {
    Serve {
        #[arg(short, long, value_name = "PORT")]
        port: Option<u16>,
    },
    Read {},
}
