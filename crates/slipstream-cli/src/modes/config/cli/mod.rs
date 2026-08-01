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

/// Output format type.
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum FetchOutputFormat {
    /// Output as json.
    #[default]
    Json,
    /// Output as toml.
    Toml,
    /// Output as markdown.
    Markdown,
    /// Custom format.
    Custom(String),
}

impl From<&str> for FetchOutputFormat {
    fn from(value: &str) -> Self {
        match value {
            "json" => Self::Json,
            "toml" => Self::Toml,
            "markdown" | "md" => Self::Markdown,
            _ => Self::Custom(value.to_string()),
        }
    }
}

// impl clap::ValueEnum for FetchOutputFormat {
//     fn value_variants<'a>() -> &'a [Self] {
//         &[
//             Self::Json,
//             Self::Toml,
//             Self::Markdown,
//             // Self::Custom("<custom>".into()),
//         ]
//     }

//     fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
//         Some(match self {
//             Self::Json => {
//                 clap::builder::PossibleValue::new("json").help("Export to json")
//             }
//             Self::Toml => {
//                 clap::builder::PossibleValue::new("toml").help("Export to toml")
//             }
//             Self::Markdown => clap::builder::PossibleValue::new("markdown")
//                 .alias("md")
//                 .help("Export to markdown"),
//             Self::Custom(p) => clap::builder::PossibleValue::new(p.as_ref())
//                 .help("Use custom jinja template"),
//         })
//     }

//     fn from_str(
//         input: &str,
//         _ignore_case: bool,
//     ) -> std::prelude::v1::Result<Self, String> {
//         Ok(match input {
//             "json" => Self::Json,
//             "toml" => Self::Toml,
//             "markdown" | "md" => Self::Markdown,
//             _ => Self::Custom(input.to_string()),
//         })
//     }
// }
