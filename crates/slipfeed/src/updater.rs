//! Feed update handling.

use std::collections::HashSet;
use std::str::FromStr;

use async_recursion::async_recursion;

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
    pub entries: EntrySet,
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
            for (id, feed) in &self.feeds {
                let sender = sender.clone();
                let id = id.clone();
                let feed = feed.clone();
                set.spawn(async move {
                    if let Err(_) = tokio::time::timeout(
                        // TODO: Make duration customizable per-feed.
                        tokio::time::Duration::from_secs(15),
                        FeedUpdater::feed_get_entries(&now, &feed, &id, sender),
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
    /// TODO: Replace syndication with just checking RSS, ATOM, etc in order.
    async fn raw_feed_get_entries(
        parse_time: &DateTime<Utc>,
        feed: &RawFeed,
        id: &FeedId,
        sender: tokio::sync::mpsc::UnboundedSender<(Entry, FeedId)>,
    ) {
        if let Ok(req_result) = reqwest::get(feed.url.as_str()).await {
            if let Ok(body) = req_result.text().await {
                if let Ok(syn) = syndication::Feed::from_str(body.as_str()) {
                    match syn {
                        syndication::Feed::Atom(atom_feed) => {
                            for entry in atom_feed.entries() {
                                let date = match DateTime::<chrono::FixedOffset>::parse_from_rfc3339(
                                    entry.updated(),
                                ) {
                                    Ok(dt) => dt.to_utc(),
                                    Err(_) => parse_time.clone(),
                                };
                                let parsed = Entry {
                                    title: entry.title().to_string(),
                                    date,
                                    author: entry.authors().iter().fold(
                                        "".to_string(),
                                        |acc, author| {
                                            format!("{} {}", acc, author.name())
                                                .to_string()
                                        },
                                    ),
                                    content: entry.content().iter().fold(
                                        "".to_string(),
                                        |_, cont| {
                                            format!(
                                                "{}",
                                                cont.value().unwrap_or("")
                                            )
                                            .to_string()
                                        },
                                    ),
                                    url: entry.links().iter().fold(
                                        "".to_string(),
                                        |_, url| {
                                            format!("{}", url.href())
                                                .to_string()
                                        },
                                    ),
                                    // tags: Vec::new(),
                                    // feeds: Vec::new(),
                                };
                                sender.send((parsed, *id)).ok();
                            }
                        }
                        syndication::Feed::RSS(rss_feed) => {
                            for entry in rss_feed.items {
                                let date = match entry.pub_date() {
                                    Some(dt) => {
                                        match DateTime::<chrono::FixedOffset>::parse_from_rfc2822(
                                            dt,
                                        ) {
                                            Ok(dt) => dt.to_utc(),
                                            Err(_) => parse_time.clone(),
                                        }
                                    }
                                    None => parse_time.clone(),
                                };
                                let parsed = Entry {
                                    title: entry
                                        .title()
                                        .unwrap_or("")
                                        .to_string(),
                                    date,
                                    author: entry
                                        .author()
                                        .unwrap_or("")
                                        .to_string(),
                                    content: entry
                                        .content()
                                        .unwrap_or("")
                                        .to_string(),
                                    url: entry.link().unwrap_or("").to_string(),
                                    // tags: Vec::new(),
                                    // feeds: Vec::new(),
                                };
                                sender.send((parsed, *id)).ok();
                            }
                        }
                    }
                }
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

    // TODO: Check cycles via max depth!
    // pub fn entry_in_feed(&self, entry: &EntrySetItem, id: FeedId) -> bool {
    //     if let Some(feed) = self.feeds.get(&id) {
    //         if entry.feeds.contains(&id) {
    //             if feed.passes_filters(&entry.entry) {
    //                 return true;
    //             }
    //         }
    //         return match &feed.underlying {
    //             UnderlyingFeed::RawFeed(_) => false,
    //             UnderlyingFeed::AggregateFeed(agg) => {
    //                 agg.feeds.iter().any(|f| self.entry_in_feed(entry, *f))
    //             }
    //         };
    //     }
    //     false
    // }

    pub fn iter<'a>(&'a self) -> EntrySetIter {
        return EntrySetIter::All {
            updater: self,
            next: 0,
        };
    }

    pub fn with_tags<'a>(&'a self, tag: impl Into<Tag>) -> EntrySetIter {
        return EntrySetIter::Tag {
            updater: self,
            tag: tag.into(),
            next: 0,
        };
    }

    pub fn from_feed<'a>(&'a self, feed: FeedId) -> EntrySetIter {
        return EntrySetIter::Feed {
            updater: self,
            feed,
            next: 0,
        };
    }
}
