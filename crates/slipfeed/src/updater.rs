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

/// Object passed to feeds.
#[derive(Clone)]
pub struct UpdaterContext {
    pub feed_id: FeedId,
    pub parse_time: DateTime,
    pub last_update: Option<DateTime>,
    pub sender: tokio::sync::mpsc::UnboundedSender<(Entry, FeedId)>,
}

/// Updater for feeds.
pub struct Updater {
    /// The feed being updated.
    feeds: HashMap<FeedId, FeedInfo>,
    /// Last update check.
    last_update_check: Option<DateTime>,
    /// Update frequency.
    freq: Duration,
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
            entries: EntrySet::new(maximum),
            next_feed_id: 0,
        }
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

    /// Update a feed. Returns early if the global frequency has not elapsed.
    pub async fn update(&mut self) {
        let span = tracing::trace_span!("slipfeed::update");
        let _enter = span.enter();
        let now = DateTime::now();

        // Return if too early for update.
        let last_check = self
            .last_update_check
            .clone()
            .unwrap_or_else(|| DateTime::epoch());
        if !last_check.has_passed(&self.freq) {
            tracing::trace!("Not time to update, skipping.");
            return;
        }

        // Perform updates.
        self.last_update_check = Some(now.clone());
        // self.entries.clear(); // TODO: Don't clear!
        let (sender, mut receiver) =
            tokio::sync::mpsc::unbounded_channel::<(Entry, FeedId)>();
        {
            tracing::info!("Updating all feeds.");
            let mut set = tokio::task::JoinSet::new();
            for (id, feed_info) in self.feeds.iter() {
                let feed_info = feed_info.clone();
                let sender = sender.clone();
                let id = id.clone();
                let feed = feed_info.feed.clone();
                let ctx = UpdaterContext {
                    feed_id: id.clone(),
                    parse_time: now.clone(),
                    last_update: feed_info.last_update.clone(),
                    sender: sender.clone(),
                };

                set.spawn(async move {
                    let mut feed = feed.write().await;
                    if let Err(_) = tokio::time::timeout(
                        feed_info.attr.timeout.to_tokio(),
                        feed.update(&ctx, &feed_info.attr),
                    )
                    .await
                    {
                        tracing::warn!("Update timed out for {:?}", feed);
                    }
                });
            }
            set.join_all().await;

            // Update parse times.
            for feed_info in self.feeds.values_mut() {
                feed_info.last_update = Some(now.clone());
            }
        }

        // Gather entries and tag.
        tracing::info!("Gathering entries.");
        while let Ok((mut entry, feed)) = receiver.try_recv() {
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
            self.feeds.len(),
        );
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
