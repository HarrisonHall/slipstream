//! CLI.

use super::*;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,
    #[arg(short, long, value_name = "PORT")]
    pub port: Option<u16>,
    #[arg(short, long, action)]
    pub debug: bool,
}

impl Cli {
    /// Parse configuration.
    pub fn parse_config(&self) -> Result<Config, Error> {
        // Get specified config path.
        let config_path: PathBuf = match &self.config {
            Some(path) => path.clone(),
            None => match PathBuf::from_str(DEFAULT_CONFIG_DIR) {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!(
                        "Invalid default config {}: {}",
                        DEFAULT_CONFIG_DIR,
                        e
                    );
                    return Err(Error::InvalidConfig);
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
            tracing::debug!("Created config file @ {:?}", config_path);
            if let Err(e) = std::fs::File::create(config_path.as_path()) {
                tracing::error!(
                    "Unable to create config file at {:?}: {}",
                    config_path,
                    e
                );
                return Err(Error::InvalidConfig);
            }
        }
        // Read file.
        let config_data = match std::fs::read_to_string(&config_path) {
            Ok(data) => data,
            Err(e) => {
                tracing::error!(
                    "Unable to read data from config file {:?}: {}",
                    config_path,
                    e
                );
                return Err(Error::InvalidConfig);
            }
        };
        // Parse.
        match Config::deserialize(toml::Deserializer::new(&config_data)) {
            Ok(config) => Ok(config),
            Err(e) => {
                tracing::error!("Configuration file is not valid: {}", e);
                Err(Error::InvalidConfig)
            }
        }
    }
}
