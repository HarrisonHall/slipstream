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

impl Updater {
    pub async fn update(&mut self) -> () {
        self.updater.update().await;
    }

    pub fn syndicate(&self, feed: &str) -> String {
        let mut syn = atom::FeedBuilder::default();
        syn.title(feed)
            .author(atom::PersonBuilder::default().name("slipknot").build());
        // if let Some(id) = self.feeds.get(feed) {
        // TODO - actually map entries by feed
        for entry in &self.updater.entries {
            syn.entry(
                atom::EntryBuilder::default()
                    .title(entry.title.clone())
                    .summary(Some(entry.content.clone().into()))
                    .link(
                        atom::LinkBuilder::default()
                            .href(entry.url.clone())
                            .title(entry.title.clone())
                            .build(),
                    )
                    .published(Some(entry.date.clone().into()))
                    .updated(entry.date.clone())
                    .build(),
            );
        }
        // }
        syn.build().to_string()
    }
}
