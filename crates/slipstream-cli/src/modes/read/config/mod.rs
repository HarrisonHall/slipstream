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
    pub bindings: HashMap<BindingKey, Commandish>,
    /// Custom commands.
    #[serde(default)]
    pub commands: Vec<CustomCommand>,
    /// How far to scroll.
    #[serde(default = "ReadConfig::default_scroll")]
    pub scroll: u8,
    /// Scroll buffer, how many lines before scrolling begins.
    #[serde(
        default = "ReadConfig::default_scroll_buffer",
        alias = "scroll-buffer"
    )]
    pub scroll_buffer: u8,
}

impl ReadConfig {
    fn default_scroll() -> u8 {
        2
    }

    fn default_scroll_buffer() -> u8 {
        3
    }

    /// Map crossterm key to reader command.
    /// This prioritizes the configured key bindings, but falls back to the
    /// defaults. If a default is not preferred, the config should specific
    /// a mapping of the key to "none".
    pub fn get_key_command(&self, key: &KeyEvent) -> Commandish {
        tracing::trace!("Key press: {:?}", key);

        for (binding, command) in self.bindings.iter() {
            if *key == binding.into() {
                return match &*command {
                    Commandish::CustomCommandRef(name) => {
                        self.get_custom_command(name.as_str())
                    }
                    _ => command.clone(),
                };
            }
        }

        if *key == UPDATE {
            Commandish::Literal(ReadCommandLiteral::Update)
        } else if *key == QUIT {
            Commandish::Literal(ReadCommandLiteral::Quit)
        } else if *key == DOWN {
            Commandish::Literal(ReadCommandLiteral::Down)
        } else if *key == UP {
            Commandish::Literal(ReadCommandLiteral::Up)
        } else if *key == LEFT {
            Commandish::Literal(ReadCommandLiteral::Left)
        } else if *key == RIGHT {
            Commandish::Literal(ReadCommandLiteral::Right)
        } else if *key == PAGE_DOWN {
            Commandish::Literal(ReadCommandLiteral::PageDown)
        } else if *key == PAGE_UP {
            Commandish::Literal(ReadCommandLiteral::PageUp)
        } else if *key == TAB {
            Commandish::Literal(ReadCommandLiteral::Swap)
        } else if *key == MENU {
            Commandish::Literal(ReadCommandLiteral::Menu)
        } else if *key == COMMAND_MODE {
            Commandish::Literal(ReadCommandLiteral::CommandMode)
        } else if *key == SEARCH_MODE {
            Commandish::Literal(ReadCommandLiteral::SearchMode)
        } else {
            Commandish::Literal(ReadCommandLiteral::None)
        }
    }

    /// Get custom command associated with a command name.
    pub fn get_custom_command(&self, name: impl AsRef<str>) -> Commandish {
        for command in &self.commands {
            if *command.name == name.as_ref() {
                return command.into();
            }
        }

        tracing::warn!(
            "Failed to get custom command by name: {}",
            name.as_ref()
        );
        Commandish::Literal(ReadCommandLiteral::None)
    }
}
