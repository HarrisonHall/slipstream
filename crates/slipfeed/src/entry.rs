//! Feed entry.

use std::collections::HashSet;

use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::*;

/// An entry from a feed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub title: String,
    pub date: DateTime<Utc>,
    pub author: String,
    pub content: String,
    pub url: String,
    // pub tags: Vec<Tag>,
    // pub feeds: Vec<FeedId>,
}

// impl PartialEq for Entry {
//     fn eq(&self, other: &Entry) -> bool {
//         self.title.eq(&other.title)
//             && self.date.eq(&other.date)
//             && self.author.eq(&other.author)
//             && self.content.eq(&other.content)
//             && self.url.eq(&other.url)
//     }
// }

// impl Eq for Entry {}

/// Tags for feeds.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tag(pub String);

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

#[derive(PartialEq, Eq)]
pub struct EntrySetItem {
    pub entry: Entry,
    pub feeds: HashSet<FeedId>,
    pub tags: HashSet<Tag>,
}

impl PartialOrd for EntrySetItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.entry.date.partial_cmp(&other.entry.date)
    }
}

impl Ord for EntrySetItem {
    fn cmp(&self, other: &EntrySetItem) -> std::cmp::Ordering {
        self.entry.date.cmp(&other.entry.date)
    }
}

pub struct EntrySet {
    entries: Vec<EntrySetItem>,
}

impl EntrySet {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn add(
        &mut self,
        entry: Entry,
        feeds: HashSet<FeedId>,
        tags: HashSet<Tag>,
    ) {
        for other in self.entries.iter_mut() {
            if other.entry == entry {
                other.feeds.extend(feeds);
                other.tags.extend(tags);
                return;
            }
        }
        self.entries.push(EntrySetItem { entry, feeds, tags });
    }

    pub fn sort(&mut self) {
        self.entries.sort();
        self.entries.reverse();
    }
}

pub enum EntrySetIter<'a> {
    All {
        updater: &'a FeedUpdater,
        next: usize,
    },
    Feed {
        updater: &'a FeedUpdater,
        feed: FeedId,
        next: usize,
    },
    Tag {
        updater: &'a FeedUpdater,
        tag: Tag,
        next: usize,
    },
}

impl<'a> Iterator for EntrySetIter<'a> {
    type Item = Entry;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            EntrySetIter::All { updater, next } => {
                for entry in &updater.entries.entries[*next..] {
                    *next += 1;
                    return Some(entry.entry.clone());
                }
            }
            EntrySetIter::Feed {
                updater,
                feed,
                next,
            } => {
                for entry in &updater.entries.entries[*next..] {
                    *next += 1;
                    if entry.feeds.contains(feed) {
                        return Some(entry.entry.clone());
                    }
                }
            }
            EntrySetIter::Tag { updater, tag, next } => {
                for entry in &updater.entries.entries[*next..] {
                    *next += 1;
                    if entry.tags.contains(tag) {
                        return Some(entry.entry.clone());
                    }
                }
                return None;
            }
        }
        None
    }
}
