//! Read mode configuration.

use super::*;

mod color;
mod command;
mod tag;

pub use color::*;
pub use command::*;
pub use tag::*;

/// Read configuration.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct ReadConfig {
    /// Tags that are hidden.
    #[serde(default)]
    pub tags: TagConfig,
    /// Configured mappings for keys to commands.
    #[serde(default)]
    pub bindings: HashMap<BindingKey, ReadCommand>,
    /// Custom commands.
    #[serde(default)]
    pub commands: Vec<CustomCommand>,
    /// How far to scroll.
    #[serde(default = "ReadConfig::default_scroll")]
    pub scroll: u8,
}

impl ReadConfig {
    fn default_scroll() -> u8 {
        2
    }

    /// Map crossterm key to reader command.
    /// This prioritizes the configured key bindings, but falls back to the
    /// defaults. If a default is not preferred, the config should specific
    /// a mapping of the key to "none".
    pub fn get_key_command(&self, key: &KeyEvent) -> ReadCommand {
        tracing::trace!("Key press: {:?}", key);

        for (binding, command) in self.bindings.iter() {
            if *key == binding.into() {
                return match &*command {
                    ReadCommand::CustomCommandRef(name) => {
                        self.get_custom_command(name.as_str())
                    }
                    _ => command.clone(),
                };
            }
        }

        if *key == UPDATE {
            ReadCommand::Literal(ReadCommandLiteral::Update)
        } else if *key == QUIT {
            ReadCommand::Literal(ReadCommandLiteral::Quit)
        } else if *key == DOWN {
            ReadCommand::Literal(ReadCommandLiteral::Down)
        } else if *key == UP {
            ReadCommand::Literal(ReadCommandLiteral::Up)
        } else if *key == LEFT {
            ReadCommand::Literal(ReadCommandLiteral::Left)
        } else if *key == RIGHT {
            ReadCommand::Literal(ReadCommandLiteral::Right)
        } else if *key == PAGE_DOWN {
            ReadCommand::Literal(ReadCommandLiteral::PageDown)
        } else if *key == PAGE_UP {
            ReadCommand::Literal(ReadCommandLiteral::PageUp)
        } else if *key == TAB {
            ReadCommand::Literal(ReadCommandLiteral::Swap)
        } else if *key == MENU {
            ReadCommand::Literal(ReadCommandLiteral::Menu)
        } else if *key == COMMAND_MODE {
            ReadCommand::Literal(ReadCommandLiteral::CommandMode)
        } else if *key == SEARCH_MODE {
            ReadCommand::Literal(ReadCommandLiteral::SearchMode)
        } else {
            ReadCommand::Literal(ReadCommandLiteral::None)
        }
    }

    /// Get custom command associated with a command name.
    pub fn get_custom_command(&self, name: impl AsRef<str>) -> ReadCommand {
        for command in &self.commands {
            if *command.name == name.as_ref() {
                return command.into();
            }
        }

        tracing::warn!(
            "Failed to get custom command by name: {}",
            name.as_ref()
        );
        ReadCommand::Literal(ReadCommandLiteral::None)
    }
}
