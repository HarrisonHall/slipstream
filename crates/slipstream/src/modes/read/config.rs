//! Read mode configuration.

use super::*;

/// Read configuration.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct ReadConfig {
    /// Configured mappings for keys to commands.
    #[serde(default)]
    pub bindings: HashMap<BindingKey, ReadCommand>,
    /// Custom commands.
    #[serde(default)]
    pub commands: Vec<CustomCommand>,
}

impl ReadConfig {
    /// Map crossterm key to reader command.
    /// This prioritizes the configured key bindings, but falls back to the
    /// defaults. If a default is not preferred, the config should specific
    /// a mapping of the key to "none".
    pub fn get_key_command(&self, key: &KeyEvent) -> ReadCommand {
        tracing::warn!("Key!: {:?}", key);

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
        } else if *key == IMPORTANT {
            ReadCommand::Literal(ReadCommandLiteral::ToggleImportant)
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

/// Custom command configuration.
/// Commands are a pair of name and the command list used in a subprocess.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomCommand {
    pub name: Arc<String>,
    pub command: Arc<Vec<String>>,
}

impl From<CustomCommand> for ReadCommand {
    fn from(value: CustomCommand) -> Self {
        ReadCommand::CustomCommandFull {
            name: value.name.clone(),
            command: value.command.clone(),
        }
    }
}

impl From<&CustomCommand> for ReadCommand {
    fn from(value: &CustomCommand) -> Self {
        ReadCommand::CustomCommandFull {
            name: value.name.clone(),
            command: value.command.clone(),
        }
    }
}

/// Read command variants.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ReadCommand {
    /// The built-in commands.
    Literal(ReadCommandLiteral),
    /// Custom command name.
    CustomCommandRef(Arc<String>),
    /// Custom command definition.
    CustomCommandFull {
        name: Arc<String>,
        command: Arc<Vec<String>>,
    },
}

/// Built-in commands.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReadCommandLiteral {
    /// Do nothing.
    #[serde(alias = "none")]
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
    /// Toggle the selection important.
    #[serde(alias = "important")]
    ToggleImportant,
}
