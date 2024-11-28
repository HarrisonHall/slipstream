//! Feed entry.

use super::*;

/// An entry from a feed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub title: String,
    pub date: DateTime<Utc>,
    pub author: String,
    pub content: String,
    pub url: String,
}

/// Entry storage.
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

/// Set of entries.
/// Entries are ordered chronologically and can be iterated
/// on based on tag/feed.
pub struct EntrySet {
    entries: Vec<EntrySetItem>,
}

impl EntrySet {
    /// Create new entry set.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Clear all entries in the set.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Add/update an entry in the set.
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

    /// Sort entries in the set.
    pub fn sort(&mut self) {
        self.entries.sort();
        self.entries.reverse();
    }
}

/// Iterator type for pulling entries from the set.
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
