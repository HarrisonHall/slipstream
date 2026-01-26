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
    AggregateTag {
        tag_allowlist: Vec<String>,
        tag_blocklist: Vec<String>,
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
}

impl From<&MastodonFeedType> for slipfeed::MastodonFeedType {
    fn from(value: &MastodonFeedType) -> Self {
        match value {
            MastodonFeedType::PublicTimeline => {
                slipfeed::MastodonFeedType::PublicTimeline
            }
            MastodonFeedType::HomeTimeline => {
                slipfeed::MastodonFeedType::HomeTimeline
            }
        }
    }
}

pub trait EntryExt {
    fn to_atom(&self, config: &Config) -> atom::Entry;
}

impl EntryExt for slipfeed::Entry {
    fn to_atom(&self, config: &Config) -> atom::Entry {
        let mut atom_entry = atom::EntryBuilder::default();
        atom_entry
            .summary(Some(self.content().clone().into()))
            // .published(Some(self.date().clone().to_chrono()))
            .updated(self.date().clone().to_chrono())
            .author(
                atom::PersonBuilder::default()
                    .name(self.author().clone())
                    .build(),
            );

        // Content can either be html or markdown.
        match config.serve.export_format {
            ExportFormat::HTML => {
                atom_entry.content(atom::Content {
                    base: None,
                    lang: None,
                    value: Some(markdown::to_html(self.content().as_str())),
                    src: None,
                    content_type: Some("html".into()),
                });
            }
            ExportFormat::Markdown => {
                atom_entry.content(atom::Content {
                    base: None,
                    lang: None,
                    value: Some(self.content().clone()),
                    src: None,
                    content_type: Some("text".into()),
                });
            }
        }

        if config.serve.show_source_in_title {
            if self.feeds().len() > 0 {
                atom_entry.title(format!(
                    "[{}] {}",
                    self.feeds()
                        .iter()
                        .map(|f| (*f.name).clone())
                        .collect::<Vec<String>>()
                        .join(", "),
                    self.title()
                ));
            } else {
                atom_entry.title(self.title().clone());
            }
        } else {
            atom_entry.title(self.title().clone());
        }
        if self.source().url != "" {
            atom_entry.link(
                atom::LinkBuilder::default()
                    .href(&self.source().url)
                    .title(Some(self.source().title.clone()))
                    .mime_type(self.source().mime_type.clone())
                    .build(),
            );
        }
        if self.comments().url != "" {
            atom_entry.link(
                atom::LinkBuilder::default()
                    .href(&self.comments().url)
                    .title(Some(self.comments().title.clone()))
                    .mime_type(self.comments().mime_type.clone())
                    .build(),
            );
        }
        for link in self.other_links() {
            atom_entry.link(
                atom::LinkBuilder::default()
                    .href(&link.url)
                    .title(Some(link.title.clone()))
                    .mime_type(link.mime_type.clone())
                    .build(),
            );
        }
        atom_entry.source({
            let mut source = atom::SourceBuilder::default();
            if let Some(icon) = self.icon() {
                source.icon(icon.url.clone());
            }
            source.build()
        });

        // Add tags.
        for tag in self.tags() {
            atom_entry.category(
                atom::CategoryBuilder::default()
                    .term(String::from(tag))
                    .build(),
            );
        }

        // Use original id.
        if let Some(source_id) = self.source_id() {
            atom_entry.id(source_id);
        }

        atom_entry.build()
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
                tracing::warn!("Empty AggregateWorld lacks feed {:?}.", feed);
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

/// A feed that matches something on the allowlist, but no the blacklist.
/// If the allowlist is empty, this checks against _all_ entries.
#[derive(Clone, Debug)]
pub struct AggregateTagFeed {
    pub allowlist: Vec<slipfeed::Tag>,
    pub blocklist: Vec<slipfeed::Tag>,
}

impl AggregateTagFeed {
    pub fn new() -> Box<Self> {
        return Box::new(Self {
            allowlist: Vec::new(),
            blocklist: Vec::new(),
        });
    }

    fn in_allowlist(&self, entry: &slipfeed::Entry) -> bool {
        if self.allowlist.len() == 0 {
            return true;
        }

        for tag in self.allowlist.iter() {
            if entry.has_tag(tag) {
                return true;
            }
        }

        false
    }

    fn in_blocklist(&self, entry: &slipfeed::Entry) -> bool {
        for tag in self.blocklist.iter() {
            if entry.has_tag(tag) {
                return true;
            }
        }

        false
    }

    fn matches(&self, entry: &slipfeed::Entry) -> bool {
        self.in_allowlist(entry) && !self.in_blocklist(entry)
    }
}

#[slipfeed::feed_trait]
impl slipfeed::Feed for AggregateTagFeed {
    async fn tag(
        &mut self,
        entry: &mut slipfeed::Entry,
        feed_id: slipfeed::FeedId,
        attr: &slipfeed::FeedAttributes,
    ) {
        if self.matches(entry) {
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
