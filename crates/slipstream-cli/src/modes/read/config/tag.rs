//! Tag configuration.

use super::*;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TagConfig {
    /// Hidden tags.
    pub hidden: Vec<String>,
    /// Tag colors, in descending order of importance.
    pub colors: Vec<TagColor>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TagColor {
    /// Tag(s) that color should apply to.
    #[serde(flatten)]
    tag: TagOrTags,
    /// Optional color for the tag.
    #[serde(default)]
    color: ColorConfig,
    /// Optional indicator for the tag.
    #[serde(default)]
    indicator: Option<FlagConfig>,
}

impl TagColor {
    pub fn matches(&self, entry: &slipfeed::Entry) -> bool {
        // If no colors or indicators are specified, don't do anything for this
        // tag.
        if !self.color.specified() || self.indicator.is_none() {
            return false;
        }

        self.tag.matches_all(entry)
    }

    #[allow(unused)]
    pub fn style(&self) -> Style {
        (&self.color).into()
    }

    pub fn apply_style(&self, style: &mut Style) {
        self.color.apply_style(style);
    }

    pub fn indicator<'a>(&'a self) -> Option<Span<'a>> {
        match &self.indicator {
            Some(indicator) => indicator.as_span(),
            None => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum TagOrTags {
    #[serde(alias = "tag")]
    Tag(String),
    #[serde(alias = "tags")]
    Tags(Vec<String>),
}

impl TagOrTags {
    fn matches_all(&self, entry: &slipfeed::Entry) -> bool {
        match self {
            TagOrTags::Tag(tag) => entry.has_tag_fuzzy(tag),
            TagOrTags::Tags(tags) => {
                tags.iter().all(|tag| entry.has_tag_fuzzy(tag))
            }
        }
    }
}
