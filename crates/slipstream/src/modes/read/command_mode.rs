//! Command parsing.
//! Clap is used to simplify the built-in command-mode command parsing.

use super::*;

/// Command parser for the reader command-mode.
#[derive(Parser)]
pub struct CommandParser {
    #[command(subcommand)]
    pub command: Command,
}

impl CommandParser {
    /// Parse a text command.
    pub fn parse_command(command: impl AsRef<str>) -> Result<Self> {
        match CommandParser::try_parse_from(
            ["__PARSER__"]
                .iter()
                .map(|i| (*i))
                .chain(command.as_ref().split(" ")),
        ) {
            Ok(command) => Ok(command),
            Err(e) => bail!("{}", e),
        }
    }
}

/// Actual, top-level command used by the parser.
#[derive(Subcommand)]
pub enum Command {
    /// Quit the reader.
    Quit,
    /// Search for latest entries.
    #[command(alias = "latest", alias = "update")]
    SearchLatest,
    /// Search for important entries.
    #[command(alias = "important")]
    SearchImportant,
    /// Search for unread entries.
    #[command(alias = "unread")]
    SearchUnread,
    /// Search for tagged entries.
    #[command(alias = "tag")]
    SearchTagged { tag: String },
    /// Search for tagged entries.
    #[command(alias = "feed", alias = "source")]
    SearchFeed { feed: String },
}
