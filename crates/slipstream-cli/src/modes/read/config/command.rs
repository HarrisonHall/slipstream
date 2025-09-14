//! Command configuration.

use super::*;

/// Custom command configuration.
/// Commands are a pair of name and the command list used in a subprocess.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomCommand {
    pub name: Arc<String>,
    pub command: Arc<Vec<String>>,
    #[serde(default = "CustomCommand::default_save")]
    pub save: bool,
}

impl CustomCommand {
    fn default_save() -> bool {
        true
    }
}

impl From<CustomCommand> for Commandish {
    fn from(value: CustomCommand) -> Self {
        Commandish::CustomCommandFull(value.clone())
    }
}

impl From<&CustomCommand> for Commandish {
    fn from(value: &CustomCommand) -> Self {
        Commandish::CustomCommandFull(value.clone())
    }
}

/// Read command variants.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum Commandish {
    /// The built-in commands.
    Literal(ReadCommandLiteral),
    /// Custom command name.
    CustomCommandRef(Arc<String>),
    /// Custom command definition.
    CustomCommandFull(CustomCommand),
}

impl<'de> Deserialize<'de> for Commandish {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;

        if text.starts_with(":") {
            return Ok(Commandish::Literal(ReadCommandLiteral::Command(
                text[1..].trim().into(),
            )));
        }

        if text.starts_with("!") {
            return Ok(Commandish::CustomCommandRef(Arc::new(
                text[1..].trim().into(),
            )));
        }

        let text = format!("\"{text}\"");
        let de = match toml::de::ValueDeserializer::parse(&text) {
            Ok(de) => de,
            Err(e) => {
                return Err(<D::Error as serde::de::Error>::custom(e));
            }
        };
        return match ReadCommandLiteral::deserialize(de) {
            Ok(literal) => Ok(Commandish::Literal(literal)),
            Err(e) => Err(<D::Error as serde::de::Error>::custom(e)),
        };
    }
}

/// Built-in commands.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReadCommandLiteral {
    /// Do nothing.
    #[serde(alias = "none", alias = "noop", alias = "Noop")]
    None,
    /// Quit the read mode.
    #[serde(alias = "quit")]
    Quit,
    /// Update the feeds.
    #[serde(alias = "update")]
    Update,
    /// Go down in the current context.
    #[serde(alias = "down")]
    Down,
    /// Go up in the current context.
    #[serde(alias = "up")]
    Up,
    /// Go left in the current context.
    #[serde(alias = "left")]
    Left,
    /// Go right in the current context.
    #[serde(alias = "right")]
    Right,
    /// Go far down in the current context.
    #[serde(alias = "page-down")]
    PageDown,
    /// Go far up in the current context.
    #[serde(alias = "page-up")]
    PageUp,
    /// Swap the current context.
    #[serde(alias = "swap")]
    Swap,
    /// Toggle the menu.
    #[serde(alias = "menu")]
    Menu,
    /// Enter command mode.
    #[serde(alias = "command-mode")]
    CommandMode,
    /// Enter search mode.
    #[serde(alias = "search-mode")]
    SearchMode,
    /// Page forwards.
    #[serde(alias = "page-forwards", alias = "next")]
    PageForwards,
    /// Page backwards.
    #[serde(alias = "page-backwards", alias = "prev", alias = "previous")]
    PageBackwards,
    /// Run a specific command_mode command.
    #[serde(alias = "command")]
    Command(String),
}
