//! Feeds.

use super::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct FeedDefinition {
    #[serde(flatten)]
    feed: RawFeed,
    tags: Option<Vec<String>>,
    #[serde(flatten)]
    filters: Filters,
    #[serde(flatten)]
    options: FeedOptions,
}

impl FeedDefinition {
    pub fn feed(&self) -> &RawFeed {
        &self.feed
    }

    pub fn tags(&self) -> &Option<Vec<String>> {
        &self.tags
    }

    pub fn filters(&self) -> &Filters {
        &self.filters
    }

    pub fn options(&self) -> &FeedOptions {
        &self.options
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RawFeed {
    Raw { url: String },
    Aggregate { feeds: Vec<String> },
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
        let mut entry = atom::EntryBuilder::default();
        entry
            .title(self.title().clone())
            .summary(Some(self.content().clone().into()))
            .published(Some(self.date().clone().into()))
            .updated(self.date().clone())
            .author(
                atom::PersonBuilder::default()
                    .name(self.author().clone())
                    .build(),
            );
        if self.source().url != "" {
            entry.link(
                atom::LinkBuilder::default()
                    .href(&self.source().url)
                    .title(Some(self.source().title.clone()))
                    .mime_type(self.source().mime_type.clone())
                    .build(),
            );
        }
        if self.comments().url != "" {
            entry.link(
                atom::LinkBuilder::default()
                    .href(&self.comments().url)
                    .title(Some(self.comments().title.clone()))
                    .mime_type(self.comments().mime_type.clone())
                    .build(),
            );
        }
        for link in self.other_links() {
            entry.link(
                atom::LinkBuilder::default()
                    .href(&link.url)
                    .title(Some(link.title.clone()))
                    .mime_type(link.mime_type.clone())
                    .build(),
            );
        }
        entry.build()
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

    pub fn syndicate_all(&self, config: &Config) -> String {
        let mut syn = atom::FeedBuilder::default();
        syn.title("All")
            .author(atom::PersonBuilder::default().name("slipknot").build());
        let mut count = 0;
        for entry in self.updater.iter() {
            if count > config.global.limits.max() {
                break;
            }
            if *entry.date() < config.global.limits.oldest() {
                continue;
            }
            if !self.passes_global_filters(&entry) {
                continue;
            }
            syn.entry(entry.as_atom());
            count += 1;
        }
        syn.build().to_string()
    }

    pub fn syndicate_feed(&self, feed: &str, config: &Config) -> String {
        let mut syn = atom::FeedBuilder::default();
        syn.title(feed)
            .author(atom::PersonBuilder::default().name("slipknot").build());
        if let (Some(id), Some(feed)) =
            (self.feeds.get(feed), config.feed(feed))
        {
            let mut count = 0;
            for entry in self.updater.from_feed(*id) {
                if count >= config.global.limits.max() {
                    break;
                }
                if count >= feed.options().max() {
                    break;
                }
                if *entry.date() < config.global.limits.oldest() {
                    continue;
                }
                if *entry.date() < feed.options().oldest() {
                    continue;
                }
                if !self.passes_global_filters(&entry) {
                    continue;
                }
                // NOTE: Individual feed filters are already checked by the underlying
                // slipfeed updater.
                syn.entry(entry.as_atom());
                count += 1;
            }
        }
        syn.build().to_string()
    }

    pub fn syndicate_tag(&self, tag: &str, config: &Config) -> String {
        let mut syn = atom::FeedBuilder::default();
        syn.title(tag)
            .author(atom::PersonBuilder::default().name("slipknot").build());
        let mut count = 0;
        for entry in self.updater.with_tags(tag) {
            if count >= config.global.limits.max() {
                break;
            }
            if *entry.date() < config.global.limits.oldest() {
                continue;
            }
            if !self.passes_global_filters(&entry) {
                continue;
            }
            syn.entry(entry.as_atom());
            count += 1;
        }
        syn.build().to_string()
    }
}
