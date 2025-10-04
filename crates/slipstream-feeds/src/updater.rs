//! Feed update handling.

use super::*;

/// Information the updater keeps about the feed.
#[derive(Clone)]
struct FeedInfo {
    id: FeedId,
    feed: Arc<RwLock<Box<dyn Feed>>>,
    attr: FeedAttributes,
    last_update: Option<DateTime>,
}

/// Object passed to feeds on update.
/// This provides meta-information to the feed.
#[derive(Clone)]
pub struct UpdaterContext {
    /// The feed id.
    pub feed_id: FeedId,
    /// The parse time.
    pub parse_time: DateTime,
    /// The last time the feed had been updated.
    pub last_update: Option<DateTime>,
    /// A sender for parsed entries.
    pub sender: tokio::sync::mpsc::UnboundedSender<(Entry, FeedRef)>,
}

/// Updater for feeds.
pub struct Updater {
    /// The feed being updated.
    feeds: HashMap<FeedId, FeedInfo>,
    /// Last update check.
    last_update_check: Option<DateTime>,
    /// Update frequency.
    freq: Duration,
    /// Number of feeds to update/fetch at a time.
    workers: usize,
    /// Current entries.
    entries: EntrySet,
    /// Next feed id.
    next_feed_id: usize,
}

impl Updater {
    /// Generate a feed updater.
    pub fn new(freq: Duration, maximum: usize) -> Self {
        Self {
            feeds: HashMap::new(),
            last_update_check: None,
            freq,
            workers: 8,
            entries: EntrySet::new(maximum),
            next_feed_id: 1,
        }
    }

    /// Set the number of workers.
    pub fn set_workers(&mut self, workers: usize) {
        self.workers = workers;
    }

    /// Add a feed.
    pub fn add_feed(
        &mut self,
        feed: Box<dyn Feed>,
        attr: FeedAttributes,
    ) -> FeedId {
        let feed_id = FeedId(self.next_feed_id);
        self.next_feed_id += 1;
        self.feeds.insert(
            feed_id,
            FeedInfo {
                id: feed_id,
                feed: Arc::new(RwLock::new(feed)),
                attr,
                last_update: None,
            },
        );
        feed_id
    }

    /// Update feeds.
    /// This is _not_ cancel-safe.
    pub async fn update(&mut self) -> EntrySet {
        let span = tracing::trace_span!("slipfeed::update");
        let _enter = span.enter();
        let now = DateTime::now();

        // Wait until time to update.
        match &self.last_update_check {
            Some(last_time) => {
                let next_time = last_time.clone() + self.freq.clone();
                tokio::time::sleep_until(next_time.to_tokio()).await;
            }
            None => {}
        };

        // Perform updates.
        self.last_update_check = Some(now.clone());
        self.entries.clear();
        let total_feeds_updated;
        let (tx, mut rx) =
            tokio::sync::mpsc::unbounded_channel::<(Entry, FeedRef)>();
        {
            tracing::info!("Workers: {}", self.workers);
            use futures::StreamExt;
            tracing::info!("Updating all feeds.");

            // Collect feeds that need to be updated.
            let feeds: Vec<(FeedId, FeedInfo)> = self
                .feeds
                .iter()
                .filter(|(_id, feed_info)| {
                    // Check update time.
                    if let (Some(last_update), Some(freq)) =
                        (&feed_info.last_update, &feed_info.attr.freq)
                    {
                        if !last_update.has_passed(freq) {
                            tracing::debug!(
                                "Skipping feed {} (last updated at {}).",
                                feed_info.attr.display_name,
                                last_update
                            );
                            return false;
                        }
                    }

                    true
                })
                .map(|(id, feed_info)| (id.clone(), feed_info.clone()))
                .collect();

            // Update parse times.
            for (id, _feed_info) in &feeds {
                if let Some(feed) = self.feeds.get_mut(id) {
                    feed.last_update = Some(now.clone());
                }
            }

            total_feeds_updated = feeds.len();

            let mut updates = tokio_stream::iter(feeds)
                .map(|(id, feed_info)| {
                    let feed_info = feed_info.clone();
                    let tx = tx.clone();
                    let id = id.clone();
                    let feed = feed_info.feed.clone();
                    let ctx = UpdaterContext {
                        feed_id: id.clone(),
                        parse_time: now.clone(),
                        last_update: feed_info.last_update.clone(),
                        sender: tx.clone(),
                    };

                    async move {
                        let mut feed = feed.write().await;
                        if let Err(_) = tokio::time::timeout(
                            feed_info.attr.timeout.to_tokio(),
                            feed.update(&ctx, &feed_info.attr),
                        )
                        .await
                        {
                            tracing::warn!("Update timed out for {:?}", feed);
                        }
                    }
                })
                .buffer_unordered(self.workers);

            // Wait for all updates.
            while let Some(_) = updates.next().await {}
        }

        // Gather entries and tag.
        tracing::info!("Gathering entries.");
        drop(tx);
        while let Some((mut entry, feed)) = rx.recv().await {
            entry.add_feed(feed);
            for feed_info in self.feeds.values_mut() {
                let mut feed = feed_info.feed.write().await;
                feed.tag(&mut entry, feed_info.id, &feed_info.attr).await;
            }
            self.entries.add(entry);
        }

        tracing::info!("{} (total) entries gathered", self.entries.len());

        // Sort entries.
        self.entries.sort();

        tracing::info!(
            "{} entries sorted from {} feeds",
            self.entries.len(),
            total_feeds_updated
        );

        self.entries.clone()
    }

    /// Iterate all entries.
    pub fn iter<'a>(&'a self) -> EntrySetIter<'a> {
        return EntrySetIter::All {
            set: &self.entries,
            next: 0,
        };
    }

    /// Iterate all entries with a tag.
    pub fn with_tags<'a>(&'a self, tag: impl Into<Tag>) -> EntrySetIter<'a> {
        return EntrySetIter::Tag {
            set: &self.entries,
            tag: tag.into(),
            next: 0,
        };
    }

    /// Iterate all entries from a feed.
    pub fn from_feed<'a>(&'a self, feed: FeedId) -> EntrySetIter<'a> {
        return EntrySetIter::Feed {
            set: &self.entries,
            feed,
            next: 0,
        };
    }
}

impl Default for Updater {
    fn default() -> Self {
        Self {
            feeds: HashMap::default(),
            workers: 8,
            last_update_check: None,
            freq: Duration::from_seconds(10),
            entries: EntrySet::new(1_000),
            next_feed_id: 0,
        }
    }
}
