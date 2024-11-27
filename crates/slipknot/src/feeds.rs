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

#[derive(Clone, Serialize, Deserialize)]
pub struct Filters {
    #[serde(alias = "exclude-title-words")]
    exclude_title_words: Option<Vec<String>>,
}

impl Filters {
    pub fn get_filters(&self) -> Vec<slipfeed::Filter> {
        let mut filters: Vec<slipfeed::Filter> = Vec::new();
        if let Some(exclusions) = &self.exclude_title_words {
            let exclusions = Arc::new(exclusions.clone());
            filters.push(Arc::new(move |_feed, entry| {
                for word in entry.title.split(" ") {
                    let word = word.to_lowercase();
                    for exclusion in exclusions.iter() {
                        let exclusion = exclusion.to_lowercase();
                        if word == exclusion {
                            return false;
                        }
                    }
                }
                true
            }));
        }
        filters
    }
}

pub struct Updater {
    pub updater: slipfeed::FeedUpdater,
    pub feeds: HashMap<String, slipfeed::FeedId>,
}

trait EntryExt {
    fn as_atom(&self) -> atom::Entry;
}

impl EntryExt for slipfeed::Entry {
    fn as_atom(&self) -> atom::Entry {
        atom::EntryBuilder::default()
            .title(self.title.clone())
            .summary(Some(self.content.clone().into()))
            .link(
                atom::LinkBuilder::default()
                    .href(self.url.clone())
                    .title(self.title.clone())
                    .build(),
            )
            .published(Some(self.date.clone().into()))
            .updated(self.date.clone())
            .build()
    }
}

impl Updater {
    pub async fn update(&mut self) -> () {
        self.updater.update().await;
    }

    pub fn syndicate_all(&self) -> String {
        let mut syn = atom::FeedBuilder::default();
        syn.title("All")
            .author(atom::PersonBuilder::default().name("slipknot").build());
        for entry in self.updater.iter() {
            syn.entry(entry.as_atom());
        }
        syn.build().to_string()
    }

    pub fn syndicate_feed(&self, feed: &str) -> String {
        let mut syn = atom::FeedBuilder::default();
        syn.title(feed)
            .author(atom::PersonBuilder::default().name("slipknot").build());
        if let Some(id) = self.feeds.get(feed) {
            for entry in self.updater.from_feed(*id) {
                syn.entry(entry.as_atom());
            }
        }
        syn.build().to_string()
    }

    pub fn syndicate_tag(&self, tag: &str) -> String {
        let mut syn = atom::FeedBuilder::default();
        syn.title(tag)
            .author(atom::PersonBuilder::default().name("slipknot").build());
        for entry in self.updater.with_tags(tag) {
            syn.entry(entry.as_atom());
        }
        syn.build().to_string()
    }
}
