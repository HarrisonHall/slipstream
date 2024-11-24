//! Feed management.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use async_recursion::async_recursion;
use bon::bon;
use chrono::DateTime;
use chrono::Duration;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::entry::Entry;
use crate::entry::Tag;

/// A filter is a function that takes a feed and entry and returns true if it passes, or
/// false if it fails.
// pub type Filter = fn(&Feed, &Entry) -> bool;
pub type Filter = Arc<dyn Fn(&Feed, &Entry) -> bool + Send + Sync>;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedId(usize);

/// Any type of feed.
#[derive(Clone)]
pub enum Feed {
    AggregateFeed(AggregateFeed),
    RawFeed(RawFeed),
}

impl Feed {
    /// Get tags for a feed.
    pub fn get_tags<'a>(&'a self) -> Box<dyn Iterator<Item = &Tag> + 'a> {
        Box::new(match self {
            Feed::AggregateFeed(feed) => feed.tags.iter(),
            Feed::RawFeed(feed) => feed.tags.iter(),
        })
    }
}

impl From<RawFeed> for Feed {
    fn from(feed: RawFeed) -> Self {
        Feed::RawFeed(feed)
    }
}

impl From<AggregateFeed> for Feed {
    fn from(feed: AggregateFeed) -> Self {
        Feed::AggregateFeed(feed)
    }
}

/// A raw feed is a direct feed from a url.
#[derive(Clone)]
pub struct RawFeed {
    pub url: String,
    pub tags: Vec<Tag>,
    pub filters: Vec<Filter>,
}

/// An aggregate feed is a collection of other feeds and filters.
#[derive(Clone)]
pub struct AggregateFeed {
    /// Other feeds in aggregate.
    pub feeds: Vec<FeedId>,
    /// Tags for feed.
    pub tags: Vec<Tag>,
    /// Filters to apply to entries.
    // #[serde(skip_serializing, skip_deserializing)]
    pub filters: Vec<Filter>,
}

#[bon]
impl AggregateFeed {
    /// Create a new aggregate feed.
    pub fn new() -> Self {
        Self {
            feeds: Vec::new(),
            tags: Vec::new(),
            filters: Vec::new(),
        }
    }

    #[builder]
    fn builder(
        feeds: Vec<FeedId>,
        tags: Vec<Tag>,
        filters: Vec<Filter>,
    ) -> Self {
        Self {
            feeds,
            tags,
            filters,
        }
    }

    /// Add a filter.
    pub(crate) fn add_filter(&mut self, filter: Filter) {
        self.filters.push(filter);
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
    /// Current entries
    pub entries: Vec<Entry>,
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
            entries: Vec::new(),
            next_feed_id: 0,
        }
    }

    /// Add a feed.
    pub fn add_feed(&mut self, feed: impl Into<Feed>) -> FeedId {
        let feed_id = FeedId(self.next_feed_id);
        self.next_feed_id += 1;
        self.feeds.insert(feed_id, feed.into());
        feed_id
    }

    /// Update a feed. Returns early if the frequency has not elapsed.
    pub async fn update(&mut self) {
        let now = Utc::now();
        let last_check = self
            .last_update_check
            .unwrap_or_else(|| DateTime::UNIX_EPOCH)
            .to_utc();
        // Return if too early for update
        if now - last_check < self.freq {
            return;
        }
        // Perform check
        self.last_update_check = Some(now);
        self.entries.clear();
        let (sender, mut receiver) =
            tokio::sync::mpsc::unbounded_channel::<Entry>();
        // TODO - parallelize
        for (_id, feed) in &self.feeds {
            self.feed_get_entries(&now, &feed, sender.clone()).await
        }
        // Gather.
        while let Ok(entry) = receiver.try_recv() {
            self.entries.push(entry);
        }
    }

    /// Get entries from a specific feed.
    #[async_recursion]
    async fn feed_get_entries(
        &self,
        parse_time: &DateTime<Utc>,
        feed: &Feed,
        sender: tokio::sync::mpsc::UnboundedSender<Entry>,
    ) {
        match feed {
            Feed::RawFeed(feed) => {
                FeedUpdater::raw_feed_get_entries(parse_time, feed, sender)
                    .await;
            }
            Feed::AggregateFeed(feed) => {
                // TODO - Parallelize this!
                // let mut entries = Vec::<Entry>::new();
                let (subsender, mut receiver) =
                    tokio::sync::mpsc::unbounded_channel::<Entry>();
                // Iterate subfeeds and get entries
                for id in feed.feeds.iter() {
                    let subfeed = self.feeds.get(id).expect("...");
                    let sender = subsender.clone();
                    self.feed_get_entries(parse_time, subfeed, sender).await;
                }
                while let Ok(mut entry) = receiver.try_recv() {
                    if feed.filters.iter().all(|filter| {
                        filter(&Feed::AggregateFeed(feed.clone()), &entry)
                    }) {
                        feed.tags
                            .iter()
                            .for_each(|tag| entry.tags.push(tag.clone()));
                        sender.send(entry).ok();
                    }
                }
            }
        }
    }

    /// Get entries from a raw feed.
    /// TODO: Replace syndication with just checking RSS, ATOM, etc in order.
    async fn raw_feed_get_entries(
        parse_time: &DateTime<Utc>,
        feed: &RawFeed,
        sender: tokio::sync::mpsc::UnboundedSender<Entry>,
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
                                    tags: feed.tags.clone(),
                                };
                                if feed.filters.iter().all(|filter| {
                                    filter(
                                        &Feed::RawFeed(feed.clone()),
                                        &parsed,
                                    )
                                }) {
                                    sender.send(parsed).ok();
                                }
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
                                    tags: feed.tags.clone(),
                                };
                                if feed.filters.iter().all(|filter| {
                                    filter(
                                        &Feed::RawFeed(feed.clone()),
                                        &parsed,
                                    )
                                }) {
                                    sender.send(parsed).ok();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
