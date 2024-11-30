//! Feeds.

use super::*;

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Feed {
    Raw {
        url: String,
        tags: Option<Vec<String>>,
        #[serde(flatten)]
        filters: Filters,
    },
    Aggregate {
        feeds: Vec<String>,
        tags: Option<Vec<String>>,
        #[serde(flatten)]
        filters: Filters,
    },
}

pub struct Updater {
    pub updater: slipfeed::FeedUpdater,
    pub feeds: HashMap<String, slipfeed::FeedId>,
    pub global_filters: Vec<slipfeed::Filter>,
}

trait EntryExt {
    fn as_atom(&self) -> atom::Entry;
}

impl EntryExt for slipfeed::Entry {
    fn as_atom(&self) -> atom::Entry {
        atom::EntryBuilder::default()
            .title(self.title().clone())
            .summary(Some(self.content().clone().into()))
            .link(
                atom::LinkBuilder::default()
                    .href(self.url().clone())
                    .title(self.title().clone())
                    .build(),
            )
            .published(Some(self.date().clone().into()))
            .updated(self.date().clone())
            .author(
                atom::PersonBuilder::default()
                    .name(self.author().clone())
                    .build(),
            )
            .build()
    }
}

impl Updater {
    pub async fn update(&mut self) -> () {
        self.updater.update().await;
    }

    pub fn passes_global_filters(&self, entry: &slipfeed::Entry) -> bool {
        let feed = slipfeed::Feed::from_raw("");
        self.global_filters.iter().all(|f| f(&feed, entry))
    }

    pub fn syndicate_all(&self) -> String {
        let mut syn = atom::FeedBuilder::default();
        syn.title("All")
            .author(atom::PersonBuilder::default().name("slipknot").build());
        for entry in self.updater.iter() {
            if self.passes_global_filters(&entry) {
                syn.entry(entry.as_atom());
            }
        }
        syn.build().to_string()
    }

    pub fn syndicate_feed(&self, feed: &str) -> String {
        let mut syn = atom::FeedBuilder::default();
        syn.title(feed)
            .author(atom::PersonBuilder::default().name("slipknot").build());
        if let Some(id) = self.feeds.get(feed) {
            for entry in self.updater.from_feed(*id) {
                if self.passes_global_filters(&entry) {
                    syn.entry(entry.as_atom());
                }
            }
        }
        syn.build().to_string()
    }

    pub fn syndicate_tag(&self, tag: &str) -> String {
        let mut syn = atom::FeedBuilder::default();
        syn.title(tag)
            .author(atom::PersonBuilder::default().name("slipknot").build());
        for entry in self.updater.with_tags(tag) {
            if self.passes_global_filters(&entry) {
                syn.entry(entry.as_atom());
            }
        }
        syn.build().to_string()
    }
}
