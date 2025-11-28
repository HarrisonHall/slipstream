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
    /// Export content format.
    #[serde(default = "ExportFormat::default")]
    pub export_format: ExportFormat,
}

impl ServeConfig {
    fn default_show_source_in_title() -> bool {
        false
    }
}

/// The export format for serving content.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExportFormat {
    #[serde(alias = "html")]
    HTML,
    #[serde(alias = "markdown")]
    Markdown,
}

impl Default for ExportFormat {
    fn default() -> Self {
        Self::HTML
    }
}
