//! Tags.

use super::*;

/// Tags for feeds.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tag(String);

impl Tag {
    /// Generate a new tag.
    pub fn new(from: impl Into<String>) -> Self {
        Self(from.into())
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialOrd for Tag {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for Tag {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl From<Tag> for String {
    fn from(value: Tag) -> String {
        value.0
    }
}

impl From<&Tag> for String {
    fn from(value: &Tag) -> String {
        value.0.clone()
    }
}

impl From<String> for Tag {
    fn from(value: String) -> Self {
        Tag(value)
    }
}

impl From<&str> for Tag {
    fn from(value: &str) -> Self {
        Tag(value.into())
    }
}
