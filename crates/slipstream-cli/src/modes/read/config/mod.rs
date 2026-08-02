//! Read mode configuration.

use super::*;

mod color;
mod command;
mod flag;
mod preview;
mod tag;

pub use color::*;
pub use command::*;
pub use flag::*;
pub use preview::*;
pub use tag::*;

/// Read configuration.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct ReadConfig {
    /// Tags that are hidden.
    #[serde(default)]
    pub tags: TagConfig,
    /// Configured mappings for keys to commands.
    #[serde(default)]
    pub bindings: BTreeMap<BindingKey, Commandish>,
    /// How far to scroll (mouse).
    #[serde(default = "ReadConfig::default_scroll")]
    pub scroll: u8,
    /// Scroll buffer, how many lines before scrolling begins.
    #[serde(
        default = "ReadConfig::default_scroll_buffer",
        alias = "scroll-buffer"
    )]
    pub scroll_buffer: u8,
    /// The initial search command.
    #[serde(
        default = "ReadConfig::default_initial_search",
        alias = "initial-search"
    )]
    pub initial_search: String,
    /// Per-entry preview format.
    #[serde(default, alias = "preview-format")]
    pub preview_format: PreviewFormat,
}

impl ReadConfig {
    /// By default, scroll 2 lines per mouse movement.
    fn default_scroll() -> u8 {
        2
    }

    /// By default, scroll before cursor moves into the top or bottom 3 rows.
    fn default_scroll_buffer() -> u8 {
        3
    }

    /// By default, no flags are used in the initial search.
    fn default_initial_search() -> String {
        "".into()
    }
}
