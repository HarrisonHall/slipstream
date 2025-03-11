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
    pub updater: slipfeed::Updater,
    pub feeds: HashMap<String, slipfeed::FeedId>,
    pub feeds_ids: HashMap<slipfeed::FeedId, String>,
    pub global_filters: Vec<slipfeed::Filter>,
    pub all_filters: Vec<slipfeed::Filter>,
}

trait EntryExt {
    fn to_atom(&self) -> atom::Entry;
}

impl EntryExt for slipfeed::Entry {
    fn to_atom(&self) -> atom::Entry {
        let mut entry = atom::EntryBuilder::default();
        entry
            .title(self.title().clone())
            .summary(Some(self.content().clone().into()))
            // .published(Some(self.date().clone().to_chrono()))
            .updated(self.date().clone().to_chrono())
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

    pub fn feed_name(&self, feed: slipfeed::FeedId) -> Option<&String> {
        self.feeds_ids.get(&feed)
    }

    pub fn passes_global_filters(&self, entry: &slipfeed::Entry) -> bool {
        let feed = NoopFeed;
        self.global_filters.iter().all(|f| f(&feed, entry))
    }

    pub fn passes_all_filters(&self, entry: &slipfeed::Entry) -> bool {
        let feed = NoopFeed;
        self.all_filters.iter().all(|f| f(&feed, entry))
    }

    pub fn syndicate_all(&self, config: &Config) -> String {
        let mut syn = atom::FeedBuilder::default();
        syn.title("All")
            .author(atom::PersonBuilder::default().name("slipstream").build());
        let mut count = 0;
        for entry in self.updater.iter() {
            if count > config.global.limits.max() {
                break;
            }
            if config.global.limits.too_old(entry.date()) {
                continue;
            }
            if !self.passes_global_filters(&entry) {
                continue;
            }
            if !self.passes_all_filters(&entry) {
                continue;
            }
            syn.entry(entry.to_atom());
            count += 1;
        }
        syn.build().to_string()
    }

    pub fn collect_all(&self, config: &Config) -> Vec<slipfeed::Entry> {
        let mut entries = Vec::with_capacity(config.global.limits.max());
        let mut count = 0;
        for entry in self.updater.iter() {
            if count > config.global.limits.max() {
                break;
            }
            if config.global.limits.too_old(entry.date()) {
                continue;
            }
            if !self.passes_global_filters(&entry) {
                continue;
            }
            if !self.passes_all_filters(&entry) {
                continue;
            }
            entries.push(entry.clone());
            count += 1;
        }
        entries
    }

    pub fn syndicate_feed(&self, feed: &str, config: &Config) -> String {
        let mut syn = atom::FeedBuilder::default();
        syn.title(feed)
            .author(atom::PersonBuilder::default().name("slipstream").build());
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
                if config.global.limits.too_old(entry.date()) {
                    continue;
                }
                if feed.options().too_old(entry.date()) {
                    continue;
                }
                if !self.passes_global_filters(&entry) {
                    continue;
                }
                // NOTE: Individual feed filters are already checked by the underlying
                // slipfeed updater.
                syn.entry(entry.to_atom());
                count += 1;
            }
        }
        syn.build().to_string()
    }

    pub fn collect_feed(
        &self,
        feed: &str,
        config: &Config,
    ) -> Vec<slipfeed::Entry> {
        let mut entries = Vec::with_capacity(config.global.limits.max());
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
                if config.global.limits.too_old(entry.date()) {
                    continue;
                }
                if feed.options().too_old(entry.date()) {
                    continue;
                }
                if !self.passes_global_filters(&entry) {
                    continue;
                }
                // NOTE: Individual feed filters are already checked by the underlying
                // slipfeed updater.
                entries.push(entry.clone());
                count += 1;
            }
        }
        entries
    }

    pub fn syndicate_tag(&self, tag: &str, config: &Config) -> String {
        let mut syn = atom::FeedBuilder::default();
        syn.title(tag)
            .author(atom::PersonBuilder::default().name("slipstream").build());
        let mut count = 0;
        for entry in self.updater.with_tags(tag) {
            if count >= config.global.limits.max() {
                break;
            }
            if config.global.limits.too_old(entry.date()) {
                continue;
            }
            if !self.passes_global_filters(&entry) {
                continue;
            }
            syn.entry(entry.to_atom());
            count += 1;
        }
        syn.build().to_string()
    }

    pub fn collect_tag(
        &self,
        tag: &str,
        config: &Config,
    ) -> Vec<slipfeed::Entry> {
        let mut entries = Vec::with_capacity(config.global.limits.max());
        let mut count = 0;
        for entry in self.updater.with_tags(tag) {
            if count >= config.global.limits.max() {
                break;
            }
            if config.global.limits.too_old(entry.date()) {
                continue;
            }
            if !self.passes_global_filters(&entry) {
                continue;
            }
            entries.push(entry.clone());
            count += 1;
        }
        entries
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
        // TODO: Use graph solver!
        self.feed_owns_entry_lim(feed, entry, 6)
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
        for id in entry.feeds().iter() {
            if *id == feed {
                return true;
            }
        }

        // Check indirect ownership.
        let feeds = match self.feed_feeds.get(&feed) {
            Some(feeds) => feeds,
            None => {
                tracing::warn!("AggregateWorld lacks feed {:?}.", feed);
                return false;
            }
        };
        feeds.iter().any(|feed_name| {
            if let Some(feed_id) = self.feed_ids.get(feed_name) {
                self.feed_owns_entry_lim(*feed_id, entry, limit - 1)
            } else {
                tracing::warn!("AggregateWorld lacks feed {}.", feed_name);
                false
            }
        })
    }
}

impl std::fmt::Debug for AggregateWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AggregateWorld").finish()
    }
}

#[derive(Clone, Debug)]
pub struct AggregateFeed {
    world: Arc<RwLock<AggregateWorld>>,
}

impl AggregateFeed {
    pub fn new(world: Arc<RwLock<AggregateWorld>>) -> Box<Self> {
        Box::new(Self { world })
    }

    async fn owns_entry(
        &self,
        id: slipfeed::FeedId,
        entry: &slipfeed::Entry,
    ) -> bool {
        self.world.read().await.feed_owns_entry(id, entry)
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
                entry.add_feed(feed_id);
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
