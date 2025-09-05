//! Command parsing.
//! Clap is used to simplify the built-in command-mode command parsing.

use super::*;

/// Command parser for the reader command-mode.
#[derive(Parser, Clone)]
pub struct CommandParser {
    #[command(subcommand)]
    pub command: Command,
}

impl CommandParser {
    /// Parse a text command.
    pub fn parse_command(command: impl AsRef<str>) -> Result<Self> {
        let mut command: String = command.as_ref().into();

        // Handle search mode:
        if command.starts_with("/") {
            command = "search ".to_string() + &command[1..];
        }

        match CommandParser::try_parse_from(
            ["__PARSER__"]
                .iter()
                .map(|i| (*i))
                .chain(command.split(" ")),
        ) {
            Ok(command) => Ok(command),
            Err(e) => bail!("{}", e),
        }
    }
}

/// Actual, top-level command used by the parser.
#[derive(Subcommand, Clone)]
pub enum Command {
    /// Quit the reader.
    Quit,
    /// Search for latest entries.
    #[command(alias = "latest", alias = "update", alias = "u", alias = "l")]
    SearchLatest,
    /// Search for specific text in entries.
    #[command(alias = "search")]
    SearchAny(SearchContext),
    /// Add a tag.
    #[command(alias = "tag", alias = "add-tag")]
    TagAdd { tag: String },
    /// Remove a tag.
    #[command(alias = "remove-tag")]
    TagRemove { tag: String },
}

#[derive(Parser, Clone)]
pub struct SearchContext {
    /// Filter by important.
    #[arg(short, long, default_value_t = false)]
    pub important: bool,
    /// Filter by unread.
    #[arg(short, long, default_value_t = false)]
    pub unread: bool,
    /// Filter by tag.
    #[arg(short, long)]
    pub tag: Option<String>,
    /// Filter by feed.
    #[arg(short, long)]
    pub feed: Option<String>,
    /// Search text.
    pub text: Option<String>,
}
