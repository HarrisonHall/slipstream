//! Grouping logic.

use super::*;

pub struct EntryGroupings {
    groups: Vec<EntryGroup>,
}

impl EntryGroupings {
    pub fn new(entries: &DatabaseEntryList, config: &ReadConfig) -> Self {
        let mut groups = Vec::new();

        for entry in entries.iter() {
            let mut group = EntryGroup::new(Vec::new());
            group.entries.push(entry.clone());
            groups.push(group);
        }

        Self { groups }
    }
}

pub struct EntryGroup {
    criteria: Vec<GroupCriteria>,
    pub entries: Vec<DatabaseEntry>,
}

impl EntryGroup {
    fn new(criteria: Vec<GroupCriteria>) -> Self {
        Self {
            criteria,
            entries: Vec::new(),
        }
    }

    fn belongs(&self, entry: &DatabaseEntry) -> bool {
        self.criteria.iter().any(|crit| crit.matches(entry))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GroupCriteria {
    #[serde(alias = "author")]
    Author(String),
    #[serde(alias = "feed")]
    Feed(String),
    #[serde(alias = "tag")]
    Tag(String),
}

impl GroupCriteria {
    fn matches(&self, entry: &DatabaseEntry) -> bool {
        match self {
            Self::Author(author) => entry
                .entry
                .author()
                .to_lowercase()
                .contains(&author.to_lowercase()),
            Self::Feed(feed) => entry.entry.feeds().iter().any(|feed_ref| {
                feed_ref.name.to_lowercase().contains(&feed.to_lowercase())
            }),
            Self::Tag(tag) => entry.entry.has_tag_fuzzy(tag),
        }
    }
}
