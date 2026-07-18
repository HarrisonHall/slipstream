//! Flag/indicator options.

use super::*;

/// Configuration for specifying color.
/// This is converted into a style, or applied
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlagConfig {
    symbol: String,
    color: Option<ColorLiteral>,
}

impl FlagConfig {
    /// Convert indicator to single-character span/flag.
    pub fn as_span<'a>(&'a self) -> Option<Span<'a>> {
        if self.symbol.chars().count() < 1 {
            return None;
        }

        let mut iter = self.symbol.char_indices();
        let end = match iter.nth(1) {
            None => self.symbol.len(),
            Some(e) => e.0,
        };

        let mut span = Span::raw(&self.symbol[..end]);
        if let Some(color) = &self.color {
            span = span.fg(color);
        }

        Some(span)
    }

    /// Convert color config into ANSI style.
    pub fn style(&self) -> Style {
        Style::from(self)
    }
}

impl From<&FlagConfig> for Style {
    fn from(value: &FlagConfig) -> Self {
        Self {
            fg: value.color.as_ref().map(|col| col.into()),
            bg: None,
            underline_color: None,
            add_modifier: ratatui::style::Modifier::empty(),
            sub_modifier: ratatui::style::Modifier::empty(),
        }
    }
}

impl From<FlagConfig> for Style {
    fn from(value: FlagConfig) -> Self {
        (&value).into()
    }
}
