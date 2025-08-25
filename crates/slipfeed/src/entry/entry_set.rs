//! Entry set storage and iteration.

use super::*;

/// Set of entries.
/// Entries are ordered chronologically and can be iterated
/// on based on tag/feed.
#[derive(Clone)]
pub struct EntrySet {
    entries: Vec<Entry>,
    max_length: usize,
}

impl EntrySet {
    /// Create new entry set.
    pub fn new(max_length: usize) -> Self {
        Self {
            entries: Vec::with_capacity(max_length),
            max_length,
        }
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Clear all entries in the set.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Add/update an entry in the set.
    pub fn add(&mut self, entry: Entry) {
        // if !self.set.contains(&entry.db_id) {
        //     self.set.insert(entry.db_id);
        //     self.entries.push(entry);
        // }
        for other in self.entries.iter_mut() {
            if *other == entry {
                for feed in entry.feeds().iter() {
                    other.add_feed(feed.clone());
                }
                for tag in entry.tags().iter() {
                    other.add_tag(tag);
                }
                return;
            }
        }
        self.entries.push(entry);
    }

    /// Sort entries in the set.
    pub fn sort(&mut self) {
        // Sort oldest to newest.
        self.entries.sort();
        // Reverse from newest to oldest.
        self.entries.reverse();
        // Truncate for specific length.
        self.entries.truncate(self.max_length);
    }

    /// Get a slice of entries.
    pub fn as_slice(&self) -> &[Entry] {
        &self.entries
    }

    /// Get a slice of entries, mutable.
    pub fn as_slice_mut(&mut self) -> &mut [Entry] {
        &mut self.entries
    }
}

/// Iterator type for pulling entries from the set.
pub enum EntrySetIter<'a> {
    All {
        set: &'a EntrySet,
        next: usize,
    },
    Feed {
        set: &'a EntrySet,
        feed: FeedId,
        next: usize,
    },
    Tag {
        set: &'a EntrySet,
        tag: Tag,
        next: usize,
    },
}

impl<'a> Iterator for EntrySetIter<'a> {
    type Item = &'a Entry;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            EntrySetIter::All { set, next } => {
                for entry in &set.entries[*next..] {
                    *next += 1;
                    return Some(entry);
                }
            }
            EntrySetIter::Feed { set, feed, next } => {
                for entry in &set.entries[*next..] {
                    *next += 1;
                    // for feed_ref in entry.feeds().iter() {
                    //     if feed_ref.id == *feed {
                    //         return Some(entry);
                    //     }
                    // }
                    // if entry.feeds().contains(feed) {
                    //     return Some(entry);
                    // }
                    if entry.is_from_feed(*feed) {
                        return Some(entry);
                    }
                }
            }
            EntrySetIter::Tag { set, tag, next } => {
                for entry in &set.entries[*next..] {
                    *next += 1;
                    if entry.tags().contains(tag) {
                        return Some(entry);
                    }
                }
                return None;
            }
        }
        None
    }
}
