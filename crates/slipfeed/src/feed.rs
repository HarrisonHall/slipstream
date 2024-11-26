//! Feed management.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use async_recursion::async_recursion;
use bon::bon;
use chrono::DateTime;
use chrono::Duration;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::*;

/// A filter is a function that takes a feed and entry and returns true if it passes, or
/// false if it fails.
// pub type Filter = fn(&Feed, &Entry) -> bool;
pub type Filter = Arc<dyn Fn(&Feed, &Entry) -> bool + Send + Sync>;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedId(usize);

pub struct Feed {
    underlying: UnderlyingFeed,
    tags: HashSet<Tag>,
    filters: Vec<Filter>,
}

// TODO: Should this be a trait?
/// Any type of feed.
#[derive(Clone)]
enum UnderlyingFeed {
    AggregateFeed(AggregateFeed),
    RawFeed(RawFeed),
}

#[bon]
impl Feed {
    /// Construct from raw feed.
    pub fn from_raw(url: impl AsRef<str>) -> Self {
        Self {
            underlying: RawFeed {
                url: url.as_ref().to_string(),
            }
            .into(),
            tags: HashSet::new(),
            filters: Vec::new(),
        }
    }

    /// Construct from aggregate feed.
    pub fn from_aggregate(feeds: Vec<FeedId>) -> Self {
        Self {
            underlying: AggregateFeed { feeds }.into(),
            tags: HashSet::new(),
            filters: Vec::new(),
        }
    }

    #[builder]
    fn builder(
        underlying: impl Into<UnderlyingFeed>,
        tags: HashSet<Tag>,
        filters: Vec<Filter>,
    ) -> Self {
        Self {
            underlying: underlying.into(),
            tags,
            filters,
        }
    }

    /// Add a filter.
    pub fn add_filter(&mut self, filter: Filter) {
        self.filters.push(filter);
    }

    /// Add a tag.
    pub fn add_tag(&mut self, tag: Tag) {
        self.tags.insert(tag);
    }

    /// Get tags for a feed.
    pub fn get_tags<'a>(&'a self) -> Box<dyn Iterator<Item = &Tag> + 'a> {
        return Box::new(self.tags.iter());
    }

    pub fn passes_filters(&self, entry: &Entry) -> bool {
        self.filters.iter().all(|filter| filter(self, entry))
    }
}

impl From<RawFeed> for UnderlyingFeed {
    fn from(feed: RawFeed) -> Self {
        UnderlyingFeed::RawFeed(feed)
    }
}

impl From<AggregateFeed> for UnderlyingFeed {
    fn from(feed: AggregateFeed) -> Self {
        UnderlyingFeed::AggregateFeed(feed)
    }
}

/// A raw feed is a direct feed from a url.
#[derive(Clone)]
pub struct RawFeed {
    pub url: String,
}

/// An aggregate feed is a collection of other feeds and filters.
#[derive(Clone)]
pub struct AggregateFeed {
    /// Other feeds in aggregate.
    pub feeds: Vec<FeedId>,
}

impl AggregateFeed {
    /// Create a new aggregate feed.
    pub fn new() -> Self {
        Self { feeds: Vec::new() }
    }

    /// Add a feed.
    pub(crate) fn add_feed(&mut self, feed: FeedId) {
        self.feeds.push(feed);
    }
}

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
        let now = Utc::now();
        let last_check = self
            .last_update_check
            .unwrap_or_else(|| DateTime::UNIX_EPOCH)
            .to_utc();
        // Return if too early for update.
        if now - last_check < self.freq {
            return;
        }
        // Perform check.
        self.last_update_check = Some(now);
        self.entries.clear();
        let (sender, mut receiver) =
            tokio::sync::mpsc::unbounded_channel::<(Entry, FeedId)>();
        // TODO: Parallelize.
        for (id, feed) in &self.feeds {
            self.feed_get_entries(&now, &feed, id, sender.clone()).await
        }
        // Gather.
        while let Ok((entry, feed)) = receiver.try_recv() {
            self.entries.add(entry, feed);
        }
        self.entries.sort();
    }

    /// Get entries from a specific feed.
    #[async_recursion]
    async fn feed_get_entries(
        &self,
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
                                };
                                sender.send((parsed, *id)).ok();
                            }
                        }
                    }
                }
            }
        }
    }

    // TODO: Check cycles via max depth!
    pub fn entry_in_feed(&self, entry: &EntrySetItem, id: FeedId) -> bool {
        if let Some(feed) = self.feeds.get(&id) {
            if entry.feeds.contains(&id) {
                if feed.passes_filters(&entry.entry) {
                    return true;
                }
            }
            return match &feed.underlying {
                UnderlyingFeed::RawFeed(_) => false,
                UnderlyingFeed::AggregateFeed(agg) => {
                    agg.feeds.iter().any(|f| self.entry_in_feed(entry, *f))
                }
            };
        }
        false
    }

    pub fn iter<'a>(&'a mut self) -> EntrySetIter {
        return EntrySetIter::All {
            updater: self,
            next: 0,
        };
    }

    pub fn with_tags<'a>(&'a mut self, tag: Tag) -> EntrySetIter {
        return EntrySetIter::Tag {
            updater: self,
            tag,
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
