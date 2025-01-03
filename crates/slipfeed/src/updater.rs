//! Feed update handling.

use std::{borrow::Borrow, collections::HashSet};

use super::*;

/// Updater for feeds.
pub struct FeedUpdater {
    /// The feed being updated.
    feeds: HashMap<FeedId, Feed>,
    /// Last update check.
    last_update_check: Option<DateTime<Utc>>,
    /// Update frequency.
    freq: Duration,
    /// Current entries.
    entries: EntrySet,
    /// Next feed id.
    next_feed_id: usize,
}

impl FeedUpdater {
    /// Generate a feed updater.
    pub fn new(freq: Duration) -> Self {
        Self {
            feeds: HashMap::new(),
            last_update_check: None,
            freq,
            entries: EntrySet::new(),
            next_feed_id: 0,
        }
    }

    /// Add a feed.
    pub fn add_feed(&mut self, feed: Feed) -> FeedId {
        let feed_id = FeedId(self.next_feed_id);
        self.next_feed_id += 1;
        self.feeds.insert(feed_id, feed);
        feed_id
    }

    /// Update a feed. Returns early if the frequency has not elapsed.
    pub async fn update(&mut self) {
        let span = tracing::trace_span!("slipfeed::update");
        let _enter = span.enter();
        let now = Utc::now();
        let last_check = self
            .last_update_check
            .unwrap_or_else(|| DateTime::UNIX_EPOCH)
            .to_utc();
        // Return if too early for update.
        if now - last_check < self.freq {
            tracing::trace!("Not time to update, skipping.");
            return;
        }
        // Perform updates.
        self.last_update_check = Some(now);
        self.entries.clear();
        let (sender, mut receiver) =
            tokio::sync::mpsc::unbounded_channel::<(Entry, FeedId)>();
        {
            tracing::info!("Updating all feeds.");
            let mut set = tokio::task::JoinSet::new();
            let mut url_specs = std::collections::HashSet::<String>::new();
            for (id, feed) in &self.feeds {
                let sender = sender.clone();
                let id = id.clone();
                let feed = feed.clone();
                match feed.underlying.borrow() {
                    UnderlyingFeed::RawFeed(raw) => {
                        if url_specs.contains(&raw.url) {
                            tracing::debug!("Skipping update of {:?}, underyling url already updated.", feed.underlying);
                            continue;
                        }
                        url_specs.insert(raw.url.clone());
                        set.spawn(async move {
                            if let Err(_) = tokio::time::timeout(
                                // TODO: Make duration customizable per-feed.
                                tokio::time::Duration::from_secs(15),
                                FeedUpdater::feed_get_entries(
                                    &now, &feed, &id, sender,
                                ),
                            )
                            .await
                            {
                                tracing::warn!(
                                    "Update timed out for {:?}",
                                    feed.underlying
                                );
                            }
                        });
                    }
                    _ => tracing::debug!(
                        "Skipping update of {:?}, as it's not a raw feed",
                        feed.underlying
                    ),
                }
            }
            set.join_all().await;
        }
        // Gather entries.
        tracing::info!("Gathering entries.");
        while let Ok((entry, feed)) = receiver.try_recv() {
            let mut feeds: HashSet<FeedId> = HashSet::new();
            let mut tags: HashSet<Tag> = HashSet::new();
            feeds.insert(feed);
            for (other_id, other_feed) in self.feeds.iter() {
                if self.entry_in_feed(&entry, feed, *other_id, 10) {
                    feeds.insert(*other_id);
                    tags.extend(other_feed.tags.clone());
                }
            }
            self.entries.add(entry, feeds, tags);
        }
        self.entries.sort();
        tracing::info!("{} entries gathered", self.entries.len());
    }

    /// Get entries from a specific feed.
    async fn feed_get_entries(
        parse_time: &DateTime<Utc>,
        feed: &Feed,
        id: &FeedId,
        sender: tokio::sync::mpsc::UnboundedSender<(Entry, FeedId)>,
    ) {
        match &feed.underlying {
            UnderlyingFeed::RawFeed(feed) => {
                FeedUpdater::raw_feed_get_entries(parse_time, feed, id, sender)
                    .await;
            }
            UnderlyingFeed::AggregateFeed(_) => {
                return;
            }
        }
    }

    /// Get entries from a raw feed.
    async fn raw_feed_get_entries(
        parse_time: &DateTime<Utc>,
        feed: &RawFeed,
        id: &FeedId,
        sender: tokio::sync::mpsc::UnboundedSender<(Entry, FeedId)>,
    ) {
        if let Ok(req_result) = reqwest::get(feed.url.as_str()).await {
            if let Ok(body) = req_result.text().await {
                let body = body.as_str();
                if let Ok(atom_feed) = body.parse::<atom_syndication::Feed>() {
                    for entry in atom_feed.entries() {
                        let parsed = EntryBuilder::new()
                            .title(entry.title().to_string())
                            .date(entry.updated().to_utc())
                            .author(entry.authors().iter().fold(
                                "".to_string(),
                                |acc, author| {
                                    format!("{} {}", acc, author.name())
                                        .to_string()
                                },
                            ))
                            .content(entry.content().iter().fold(
                                "".to_string(),
                                |_, cont| {
                                    format!("{}", cont.value().unwrap_or(""))
                                        .to_string()
                                },
                            ))
                            .url(
                                entry.links().iter().fold(
                                    "".to_string(),
                                    |_, url| {
                                        format!("{}", url.href()).to_string()
                                    },
                                ),
                            )
                            .build();
                        sender.send((parsed, *id)).ok();
                    }
                    return;
                }
                if let Ok(rss_feed) = body.parse::<rss::Channel>() {
                    for entry in rss_feed.items {
                        let parsed = EntryBuilder::new()
                            .title(entry.title().unwrap_or("").to_string())
                            .date(FeedUpdater::parse_time(
                                entry.pub_date().unwrap_or(""),
                                Some(parse_time),
                            ))
                            .author(entry.author().unwrap_or("").to_string())
                            .content(entry.content().unwrap_or("").to_string())
                            .url(entry.link().unwrap_or("").to_string())
                            .build();
                        sender.send((parsed, *id)).ok();
                    }
                    return;
                }
                tracing::warn!(
                    "Unable to parse feed {:?} as atom or rss",
                    feed
                );
            }
        }
    }

    pub fn entry_in_feed(
        &self,
        entry: &Entry,
        original: FeedId,
        other: FeedId,
        remaining_depth: usize,
    ) -> bool {
        if remaining_depth == 0 {
            return false;
        }
        if let Some(original_feed) = self.feeds.get(&original) {
            if original == other {
                return original_feed.passes_filters(entry);
            }
            if let Some(other_feed) = self.feeds.get(&other) {
                return match &other_feed.underlying {
                    UnderlyingFeed::RawFeed(_) => false,
                    UnderlyingFeed::AggregateFeed(agg) => {
                        agg.feeds.iter().any(|f| {
                            let mut in_downfeed = agg.feeds.contains(&original);
                            in_downfeed |= self.entry_in_feed(
                                entry,
                                original,
                                *f,
                                remaining_depth - 1,
                            );
                            in_downfeed && other_feed.passes_filters(entry)
                        })
                    }
                };
            }
        }
        false
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

    /// Attempt to parse time, defaulting to now or the current time.
    fn parse_time(
        date: impl AsRef<str>,
        now: Option<&DateTime<Utc>>,
    ) -> DateTime<Utc> {
        // rfc3339
        if let Ok(parsed) =
            DateTime::<chrono::FixedOffset>::parse_from_rfc3339(date.as_ref())
        {
            return parsed.to_utc();
        }
        // rfc2822
        if let Ok(parsed) =
            DateTime::<chrono::FixedOffset>::parse_from_rfc2822(date.as_ref())
        {
            return parsed.to_utc();
        }
        // iso8601 (at least, try)
        if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(
            date.as_ref(),
            "%Y-%m-%dT%H:%M:%SZ",
        ) {
            return DateTime::from_naive_utc_and_offset(parsed, Utc);
        }
        if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(
            date.as_ref(),
            "%Y-%m-%dT%H:%MZ",
        ) {
            return DateTime::from_naive_utc_and_offset(parsed, Utc);
        }
        if let Ok(parsed) =
            chrono::NaiveDate::parse_from_str(date.as_ref(), "%Y-%m-%d")
        {
            if let Some(parsed) = parsed.and_hms_opt(0, 0, 0) {
                return DateTime::from_naive_utc_and_offset(parsed, Utc);
            }
        }
        // Otherwise, warn and use current time.
        if !date.as_ref().is_empty() {
            tracing::warn!("Invalid timestamp: `{}`", date.as_ref());
        }
        match now {
            Some(now) => now.clone(),
            None => Utc::now(),
        }
    }
}
