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
}
