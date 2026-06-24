//! Slipstream cli utils.

use super::*;

mod config;
mod export;

pub use config::*;
pub use export::*;

/// Slipstream util mode.
#[derive(Clone, Subcommand)]
pub enum UtilMode {
    /// Manage configuration.
    Config {
        #[command(subcommand)]
        config_mode: ConfigMode,
    },
    /// Export search results.
    Export {
        #[command(subcommand)]
        search: crate::modes::read::command_mode::SearchContext,
    },
}
