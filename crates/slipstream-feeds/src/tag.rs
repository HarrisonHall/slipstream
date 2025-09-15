//! Tags.

use super::*;

/// Tags for feeds.
/// Tags are lower-case identifiers that can be fuzzy-matched.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tag(String);

impl Tag {
    /// Generate a new tag.
    pub fn new(from: impl Into<String>) -> Self {
        Self(String::from(from.into()).to_lowercase())
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
        Tag(value.to_lowercase())
    }
}

impl From<&str> for Tag {
    fn from(value: &str) -> Self {
        Tag(String::from(value).to_lowercase())
    }
}

impl AsRef<String> for Tag {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

impl AsRef<str> for Tag {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
