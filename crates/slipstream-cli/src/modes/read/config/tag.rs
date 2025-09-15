//! Tag configuration.

use super::*;

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
        entry.has_tag_fuzzy(&self.tag)
    }

    #[allow(unused)]
    pub fn style(&self) -> Style {
        (&self.color).into()
    }

    pub fn apply_style(&self, style: &mut Style) {
        self.color.apply_style(style);
    }
}
