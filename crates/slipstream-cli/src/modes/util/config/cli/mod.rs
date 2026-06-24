//! Config mode configuration.

use super::*;

/// Slipstream config mode.
#[derive(Clone, Subcommand)]
pub enum ConfigMode {
    /// Verify configuration.
    Verify,
    /// Export from current configuration.
    Export {
        /// Conversion destination.
        config_type: ConfigDestination,
        /// Conversion .
        out_file: std::path::PathBuf,
    },
    /// Import into current configuration.
    Import {
        /// Conversion destination.
        in_type: ConfigDestination,
        /// Other file location.
        in_file: std::path::PathBuf,
        /// Conversion .
        out_file: std::path::PathBuf,
    },
}

/// Slipstream destination type.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum ConfigDestination {
    Slipstream,
    Opml,
    List,
}
