//! Feeds.

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[allow(unused)]
    pub fn from_feed(feed: RawFeed) -> Self {
        Self {
            feed,
            tags: None,
            filters: Filters::default(),
            options: FeedOptions::default(),
        }
    }

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RawFeed {
    Raw {
        url: String,
    },
    Aggregate {
        feeds: Vec<String>,
    },
    MastodonStatuses {
        mastodon: String,
        #[serde(alias = "type")]
        feed_type: MastodonFeedType,
        token: Option<String>,
    },
    MastodonUserStatuses {
        mastodon: String,
        #[serde(alias = "type")]
        user: String,
        token: Option<String>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MastodonFeedType {
    #[serde(alias = "public-timeline", alias = "public")]
    PublicTimeline,
    #[serde(alias = "home-timeline", alias = "home", alias = "timeline")]
    HomeTimeline,
    // #[serde(alias = "user", alias = "user-status", alias = "user-statuses")]
    // UserStatuses(String),
}

impl From<&MastodonFeedType> for slipstream_feeds::MastodonFeedType {
    fn from(value: &MastodonFeedType) -> Self {
        match value {
            MastodonFeedType::PublicTimeline => {
                slipstream_feeds::MastodonFeedType::PublicTimeline
            }
            MastodonFeedType::HomeTimeline => {
                slipstream_feeds::MastodonFeedType::HomeTimeline
            } // MastodonFeedType::UserStatuses(user) => {
              //     slipstream_feeds::MastodonFeedType::UserStatuses {
              //         user: user.clone(),
              //     }
              // }
        }
    }
}

pub trait EntryExt {
    fn to_atom(&self, config: &Config) -> atom::Entry;
}

impl EntryExt for slipfeed::Entry {
    fn to_atom(&self, config: &Config) -> atom::Entry {
        let mut entry = atom::EntryBuilder::default();
        entry
            .summary(Some(self.content().clone().into()))
            // .published(Some(self.date().clone().to_chrono()))
            .updated(self.date().clone().to_chrono())
            .author(
                atom::PersonBuilder::default()
                    .name(self.author().clone())
                    .build(),
            );
        entry.content(atom::Content {
            base: None,
            lang: None,
            value: Some(self.content().clone()),
            src: None,
            content_type: Some("text".into()),
        });
        if config.serve.show_source_in_title {
            if self.feeds().len() > 0 {
                entry.title(format!(
                    "[{}] {}",
                    self.feeds()
                        .iter()
                        .map(|f| (*f.name).clone())
                        .collect::<Vec<String>>()
                        .join(", "),
                    self.title()
                ));
            } else {
                entry.title(self.title().clone());
            }
        } else {
            entry.title(self.title().clone());
        }
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
        for tag in self.tags() {
            entry.category(
                atom::CategoryBuilder::default()
                    .term(String::from(tag))
                    .build(),
            );
        }
        entry.id("...");
        entry.build()
    }
}

pub use slipfeed::StandardSyndication as StandardFeed;

pub struct AggregateWorld {
    /// Map of feed name to id.
    feed_ids: HashMap<String, slipfeed::FeedId>,
    /// Map of feed id to name.
    feed_names: HashMap<slipfeed::FeedId, String>,
    /// Map of feed id to aggregates.
    feed_feeds: HashMap<slipfeed::FeedId, Vec<String>>,
}

impl AggregateWorld {
    pub fn new() -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            feed_ids: HashMap::new(),
            feed_names: HashMap::new(),
            feed_feeds: HashMap::new(),
        }))
    }

    pub fn insert(
        &mut self,
        name: impl Into<String>,
        id: slipfeed::FeedId,
        aggs: Option<Vec<String>>,
    ) {
        let name = name.into();
        self.feed_ids.insert(name.clone(), id);
        self.feed_names.insert(id, name);
        self.feed_feeds
            .insert(id, aggs.unwrap_or_else(|| Vec::new()));
    }

    fn feed_owns_entry(
        &self,
        feed: slipfeed::FeedId,
        entry: &slipfeed::Entry,
    ) -> bool {
        // FUTURE: Use graph solver!
        return self.feed_owns_entry_lim(feed, entry, 6);
    }

    fn feed_owns_entry_lim(
        &self,
        feed: slipfeed::FeedId,
        entry: &slipfeed::Entry,
        limit: u8,
    ) -> bool {
        // If we're out, we're out.
        if limit == 0 {
            return false;
        }

        // Check direct ownership.
        if entry.is_from_feed(feed) {
            return true;
        }

        // Check indirect ownership.
        let feeds = match self.feed_feeds.get(&feed) {
            Some(feeds) => feeds,
            None => {
                tracing::warn!("AggregateWorld lacks feed {:?}.", feed);
                return false;
            }
        };
        return feeds.iter().any(|feed_name| {
            if let Some(feed_id) = self.feed_ids.get(feed_name) {
                self.feed_owns_entry_lim(*feed_id, entry, limit - 1)
            } else {
                tracing::warn!("AggregateWorld lacks feed {}.", feed_name);
                false
            }
        });
    }
}

impl std::fmt::Debug for AggregateWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return f.debug_struct("AggregateWorld").finish();
    }
}

#[derive(Clone, Debug)]
pub struct AggregateFeed {
    world: Arc<RwLock<AggregateWorld>>,
}

impl AggregateFeed {
    pub fn new(world: Arc<RwLock<AggregateWorld>>) -> Box<Self> {
        return Box::new(Self { world });
    }

    async fn owns_entry(
        &self,
        id: slipfeed::FeedId,
        entry: &slipfeed::Entry,
    ) -> bool {
        return self.world.read().await.feed_owns_entry(id, entry);
    }
}

#[slipfeed::feed_trait]
impl slipfeed::Feed for AggregateFeed {
    async fn tag(
        &mut self,
        entry: &mut slipfeed::Entry,
        feed_id: slipfeed::FeedId,
        attr: &slipfeed::FeedAttributes,
    ) {
        if self.owns_entry(feed_id, entry).await {
            if attr.passes_filters(self, entry) {
                for tag in attr.get_tags() {
                    entry.add_tag(tag);
                }
                entry.add_feed(slipfeed::FeedRef {
                    id: feed_id,
                    name: attr.display_name.clone(),
                });
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct NoopFeed;

#[slipfeed::feed_trait]
impl slipfeed::Feed for NoopFeed {
    async fn tag(
        &mut self,
        _entry: &mut slipfeed::Entry,
        _feed_id: slipfeed::FeedId,
        _attr: &slipfeed::FeedAttributes,
    ) {
        // Do nothing.
    }
}
