//! Read mode configuration.

use super::*;

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
}

impl ReadConfig {
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TagConfig {
    /// Hidden tags.
    pub hidden: Vec<String>,
    /// Tag colors, in descending order of importance.
    pub colors: Vec<TagColor>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TagColor {
    /// Tag that color should apply to.
    tag: String,
    /// Color for the tag.
    color: ColorConfig,
}

impl TagColor {
    pub fn matches(&self, entry: &slipfeed::Entry) -> bool {
        entry.has_tag_loose(&self.tag)
    }

    pub fn style(&self) -> Style {
        (&self.color).into()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ColorConfig {
    fg: Option<ColorLiteral>,
    bg: Option<ColorLiteral>,
    underline: Option<ColorLiteral>,
}

impl From<&ColorConfig> for Style {
    fn from(value: &ColorConfig) -> Self {
        let mut modi = ratatui::style::Modifier::empty();
        if value.underline.is_some() {
            modi = modi.union(ratatui::style::Modifier::UNDERLINED);
        }

        return Self {
            fg: match &value.fg {
                Some(col) => Some(col.into()),
                None => None,
            },
            bg: match &value.bg {
                Some(col) => Some(col.into()),
                None => None,
            },
            underline_color: match &value.underline {
                Some(col) => Some(col.into()),
                None => None,
            },
            add_modifier: modi,
            sub_modifier: ratatui::style::Modifier::empty(),
        };
    }
}

impl From<ColorConfig> for Style {
    fn from(value: ColorConfig) -> Self {
        (&value).into()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ColorLiteral {
    #[serde(alias = "black")]
    Black,
    #[serde(alias = "white")]
    White,
    #[serde(alias = "red")]
    Red,
    #[serde(alias = "blue")]
    Blue,
    #[serde(alias = "lightblue", alias = "light-blue")]
    LightBlue,
    #[serde(alias = "cyan")]
    Cyan,
    #[serde(alias = "green")]
    Green,
    #[serde(alias = "yellow")]
    Yellow,
}

impl From<&ColorLiteral> for Color {
    fn from(value: &ColorLiteral) -> Self {
        match *value {
            ColorLiteral::Black => Color::Black,
            ColorLiteral::White => Color::White,
            ColorLiteral::Red => Color::Red,
            ColorLiteral::Blue => Color::Blue,
            ColorLiteral::LightBlue => Color::LightBlue,
            ColorLiteral::Cyan => Color::Cyan,
            ColorLiteral::Green => Color::Green,
            ColorLiteral::Yellow => Color::Yellow,
        }
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
#[derive(Clone, Debug, Serialize)]
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

impl<'de> Deserialize<'de> for ReadCommand {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;

        if text.starts_with(":") {
            return Ok(ReadCommand::Literal(ReadCommandLiteral::Command(
                text[1..].trim().into(),
            )));
        }

        if text.starts_with("!") {
            return Ok(ReadCommand::CustomCommandRef(Arc::new(
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
            Ok(literal) => Ok(ReadCommand::Literal(literal)),
            Err(e) => Err(<D::Error as serde::de::Error>::custom(e)),
        };
    }
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
    /// Enter command mode.
    #[serde(alias = "command-mode")]
    CommandMode,
    /// Enter search mode.
    #[serde(alias = "search-mode")]
    SearchMode,
    /// Run a specific command_mode command.
    #[serde(alias = "command")]
    Command(String),
}
