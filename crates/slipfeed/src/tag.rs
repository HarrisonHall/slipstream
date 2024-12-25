//! Tags.

use super::*;

/// Tags for feeds.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tag(String);

impl Tag {
    /// Generate a new tag.
    #[allow(dead_code)]
    fn new(from: impl Into<String>) -> Self {
        Self(from.into())
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
