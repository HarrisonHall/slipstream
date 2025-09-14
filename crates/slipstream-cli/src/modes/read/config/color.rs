//! Color support and configuration.

use super::*;

/// Configuration for specifying color.
/// This is converted into a style, or applied
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ColorConfig {
    fg: Option<ColorLiteral>,
    bg: Option<ColorLiteral>,
    underline: Option<ColorLiteral>,
}

impl ColorConfig {
    /// Apply active parts of this color configuration to an existing style.
    pub fn apply_style(&self, style: &mut Style) {
        if let Some(fg) = &self.fg {
            *style = style.fg(Color::from(fg));
        }
        if let Some(bg) = &self.bg {
            *style = style.bg(Color::from(bg));
        }
        if let Some(underline) = &self.underline {
            *style = style.underlined();
            *style = style.underline_color(Color::from(underline));
        }
    }

    /// Convert color config into ANSI style.
    #[allow(unused)]
    pub fn style(&self) -> Style {
        Style::from(self)
    }
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

/// A literal color value used for parsing.
/// This only supports ANSI colors.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ColorLiteral {
    #[serde(alias = "black")]
    Black,
    #[serde(alias = "red")]
    Red,
    #[serde(alias = "green")]
    Green,
    #[serde(alias = "yellow")]
    Yellow,
    #[serde(alias = "blue")]
    Blue,
    #[serde(alias = "magenta")]
    Magenta,
    #[serde(alias = "cyan")]
    Cyan,
    #[serde(alias = "white")]
    White,
    #[serde(
        alias = "brightblack",
        alias = "bright-black",
        alias = "lightblack",
        alias = "light-black",
        alias = "gray"
    )]
    BrightBlack,
    #[serde(
        alias = "brightred",
        alias = "bright-red",
        alias = "lightred",
        alias = "light-red"
    )]
    BrightRed,
    #[serde(
        alias = "brightgreen",
        alias = "bright-green",
        alias = "lightgreen",
        alias = "light-green"
    )]
    BrightGreen,
    #[serde(
        alias = "brightyellow",
        alias = "bright-yellow",
        alias = "lightyellow",
        alias = "light-yellow"
    )]
    BrightYellow,
    #[serde(
        alias = "brightblue",
        alias = "bright-blue",
        alias = "lightblue",
        alias = "light-blue"
    )]
    BrightBlue,
    #[serde(
        alias = "brightmagenta",
        alias = "bright-magenta",
        alias = "lightmagenta",
        alias = "light-magenta"
    )]
    BrightMagenta,
    #[serde(
        alias = "brightcyan",
        alias = "bright-cyan",
        alias = "lightcyan",
        alias = "light-cyan"
    )]
    BrightCyan,
    #[serde(
        alias = "brightwhite",
        alias = "bright-white",
        alias = "lightwhite",
        alias = "light-white"
    )]
    BrightWhite,
}

impl From<&ColorLiteral> for Color {
    fn from(value: &ColorLiteral) -> Self {
        match *value {
            ColorLiteral::Black => Color::Black,
            ColorLiteral::Red => Color::Red,
            ColorLiteral::Green => Color::Green,
            ColorLiteral::Yellow => Color::Yellow,
            ColorLiteral::Blue => Color::Blue,
            ColorLiteral::Magenta => Color::Magenta,
            ColorLiteral::Cyan => Color::Cyan,
            ColorLiteral::White => Color::Gray,
            ColorLiteral::BrightBlack => Color::DarkGray,
            ColorLiteral::BrightRed => Color::LightRed,
            ColorLiteral::BrightGreen => Color::LightGreen,
            ColorLiteral::BrightYellow => Color::LightYellow,
            ColorLiteral::BrightBlue => Color::LightBlue,
            ColorLiteral::BrightMagenta => Color::LightMagenta,
            ColorLiteral::BrightCyan => Color::LightCyan,
            ColorLiteral::BrightWhite => Color::White,
        }
    }
}
