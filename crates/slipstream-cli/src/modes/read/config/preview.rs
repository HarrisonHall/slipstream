//! Preview options.

use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreviewFormat(Vec<PreviewToken>);

impl PreviewFormat {
    pub fn layout(&self) -> Layout {
        let mut constraints = Vec::new();
        for token in &self.0 {
            match token {
                PreviewToken::Summary => {
                    constraints.push(Constraint::Fill(1));
                }
                PreviewToken::Flags => {
                    constraints.push(Constraint::Max(4));
                }
                PreviewToken::Feed => {
                    constraints.push(Constraint::Max(12));
                }
            }
        }

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(&constraints)
    }
}

impl Default for PreviewFormat {
    fn default() -> Self {
        Self(vec![
            PreviewToken::Feed,
            PreviewToken::Flags,
            PreviewToken::Summary,
        ])
    }
}

impl std::ops::Deref for PreviewFormat {
    type Target = Vec<PreviewToken>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Configuration for specifying color.
/// This is converted into a style, or applied
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PreviewToken {
    #[serde(alias = "summary")]
    Summary,
    #[serde(alias = "flags")]
    Flags,
    #[serde(alias = "feed")]
    Feed,
    // #[serde(alias = "date")]
    // Date,
}
