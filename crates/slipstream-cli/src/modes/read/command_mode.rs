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
        let mut command: String = command.as_ref().trim().into();

        // Handle search mode:
        if command.starts_with("/") {
            command = "search ".to_string() + &command[1..];
        }

        // Handle custom command mode:
        if command.starts_with("!") {
            command = "command ".to_string() + &command[1..];
        }

        match shlex::split(&format!("__PARSER__ {}", &command)) {
            Some(split) => match CommandParser::try_parse_from(split.clone()) {
                Ok(command) => Ok(command),
                Err(e) => bail!("{}", e),
            },
            None => bail!("Failed to split command."),
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
    /// Use live-view.
    #[command(alias = "live", alias = "live-view")]
    SearchLive,
    /// Add a tag.
    #[command(alias = "tag", alias = "add-tag")]
    TagAdd { tag: String },
    /// Remove a tag.
    #[command(alias = "untag", alias = "remove-tag")]
    TagRemove { tag: String },
    /// Toggle a tag.
    #[command(alias = "toggle-tag")]
    TagToggle { tag: String },
    /// Run a user-defined command.
    #[command(alias = "run")]
    Command { command: String },
    /// Page forwards.
    #[command(alias = "next")]
    PageForwards,
    /// Page backwards.
    #[command(alias = "prev", alias = "previous")]
    PageBackwards,
}

#[derive(Parser, Clone)]
pub struct SearchContext {
    /// Filter by tag.
    #[arg(short, long, value_parser, num_args = 1.., value_delimiter = ' ')]
    pub tag: Vec<String>,
    /// Filter by not tag.
    #[arg(long, value_parser, num_args = 1.., value_delimiter = ' ')]
    pub not_tag: Vec<String>,
    /// Filter by feed.
    #[arg(short, long, value_parser, num_args = 1.., value_delimiter = ' ')]
    pub feed: Vec<String>,
    /// Filter by not feed.
    #[arg(long, value_parser, num_args = 1.., value_delimiter = ' ')]
    pub not_feed: Vec<String>,
    /// Filter by command.
    #[arg(short, long, value_parser, num_args = 1.., value_delimiter = ' ')]
    pub command: Vec<String>,
    /// Filter by not command.
    #[arg(long, value_parser, num_args = 1.., value_delimiter = ' ')]
    pub not_command: Vec<String>,
    /// Use a raw SQL clause (e.g., "UPPER(entries.author) = 'BBC-NEWS'").
    /// WARNING: This is purposefully not checked.
    #[arg(short, long, value_parser, num_args = 1..)]
    pub raw: Vec<String>,
    /// Search text.
    pub text: Option<String>,
}
