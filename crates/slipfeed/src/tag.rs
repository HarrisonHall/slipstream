//! Tags.

use super::*;

/// Tags for feeds.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tag(String);

impl Tag {
    /// Generate a new tag.
    fn new(from: impl AsRef<str>) -> Self {
        Self(from.as_ref().to_string())
    }
}

impl Into<Tag> for String {
    fn into(self) -> Tag {
        Tag(self)
    }
}

impl Into<Tag> for &str {
    fn into(self) -> Tag {
        Tag(self.to_string())
    }
}

impl Into<String> for Tag {
    fn into(self) -> String {
        self.0.clone()
    }
}
