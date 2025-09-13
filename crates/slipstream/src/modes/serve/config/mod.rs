//! Serve mode configuration.

use super::*;

/// Read configuration.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct ServeConfig {
    /// Port.
    pub port: Option<u16>,
    /// All configuration.
    pub all: Option<GlobalConfig>,
    /// Cache duration.
    #[serde(default, with = "humantime_serde::option")]
    pub cache: Option<std::time::Duration>,
    /// Put source into served title.
    #[serde(default = "ServeConfig::default_show_source_in_title")]
    pub show_source_in_title: bool,
}

impl ServeConfig {
    fn default_show_source_in_title() -> bool {
        false
    }
}
