//! Feed entry.

use chrono::DateTime;
use chrono::Utc;

/// An entry from a feed.
#[derive(Clone, Debug)]
pub struct Entry {
    pub title: String,
    pub date: DateTime<Utc>,
    pub author: String,
    pub content: String,
    pub url: String,
    pub tags: Vec<Tag>,
}

/// Tags for feeds.
#[derive(Clone, Debug)]
pub struct Tag(String);

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
