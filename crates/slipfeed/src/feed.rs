//! Feed management.

// use core::slice::SlicePattern;
use std::str::FromStr;

use async_recursion::async_recursion;
use bon::bon;
use bon::builder;
use chrono::DateTime;
use chrono::Duration;
use chrono::Utc;

use crate::entry::Entry;

/// Any type of feed.
#[derive(Clone)]
pub enum AnyFeed {
    AggregateFeed(AggregateFeed),
    Feed(Feed),
}

impl AnyFeed {
    /// Get tags for a feed.
    fn get_tags(&self) -> &[Tag] {
        match self {
            AnyFeed::AggregateFeed(feed) => feed.tags.as_slice(),
            AnyFeed::Feed(feed) => feed.tags.as_slice(),
        }
    }
}

/// Tags for feeds.
#[derive(Clone)]
pub struct Tag(String);

/// A filter is a function that takes a feed and entry and returns true if it passes, or
/// false if it fails.
pub type Filter = fn(&AnyFeed, &Entry) -> bool;

#[derive(Clone)]
pub struct Feed {
    pub name: String,
    pub url: String,
    pub tags: Vec<Tag>,
}

impl Into<AnyFeed> for Feed {
    fn into(self) -> AnyFeed {
        AnyFeed::Feed(self)
    }
}

/// An aggregate feed is a collection of other feeds and filters.
#[derive(Clone)]
pub struct AggregateFeed {
    /// Other feeds in aggregate.
    pub feeds: Vec<AnyFeed>,
    /// Tags for feed.
    pub tags: Vec<Tag>,
    /// Filters to apply to entries.
    pub filters: Vec<Filter>,
}

#[bon]
impl AggregateFeed {
    /// Create a new named pipe.
    pub fn new() -> Self {
        Self {
            feeds: Vec::new(),
            tags: Vec::new(),
            filters: Vec::new(),
        }
    }

    #[builder]
    fn builder(feeds: Vec<AnyFeed>, tags: Vec<Tag>, filters: Vec<Filter>) -> Self {
        Self {
            feeds,
            tags,
            filters,
        }
    }

    #[builder]
    pub fn updater(&mut self, frequency: Duration) -> FeedUpdater {
        FeedUpdater::new(AnyFeed::AggregateFeed(self.clone()), frequency)
    }

    /// Add a filter.
    pub(crate) fn add_filter(&mut self, filter: Filter) {
        self.filters.push(filter);
    }

    /// Add a feed.
    pub(crate) fn add_feed(&mut self, feed: impl Into<AnyFeed>) {
        self.feeds.push(feed.into());
    }
}

impl Into<AnyFeed> for AggregateFeed {
    fn into(self) -> AnyFeed {
        AnyFeed::AggregateFeed(self)
    }
}

/// Updater for a feed.
pub struct FeedUpdater {
    /// The feed being updated.
    feed: AnyFeed,
    /// Last update check.
    last_update_check: Option<DateTime<Utc>>,
    /// Update frequency.
    freq: Duration,
    /// Current entries
    entries: Vec<Entry>,
}

impl FeedUpdater {
    fn new(feed: AnyFeed, freq: Duration) -> Self {
        Self {
            feed,
            last_update_check: None,
            freq,
            entries: Vec::new(),
        }
    }

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
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<Entry>();
        FeedUpdater::feed_get_entries(&self.feed, sender).await;
        while let Ok(entry) = receiver.try_recv() {
            self.entries.push(entry);
        }
    }

    #[async_recursion]
    async fn feed_get_entries(feed: &AnyFeed, sender: tokio::sync::mpsc::UnboundedSender<Entry>) {
        match feed {
            AnyFeed::Feed(feed) => {
                FeedUpdater::raw_feed_get_entries(feed, sender).await;
            }
            AnyFeed::AggregateFeed(feed) => {
                // TODO - Parallelize this!
                // let mut entries = Vec::<Entry>::new();
                let (subsender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<Entry>();
                // Iterate subfeeds and get entries
                for subfeed in feed.feeds.iter() {
                    let sender = subsender.clone();
                    FeedUpdater::feed_get_entries(subfeed, sender).await;
                }
                while let Ok(entry) = receiver.try_recv() {
                    if feed
                        .filters
                        .iter()
                        .all(|filter| filter(&AnyFeed::AggregateFeed(feed.clone()), &entry))
                    {
                        sender.send(entry).ok();
                    }
                }
            }
        }
    }

    async fn raw_feed_get_entries(
        feed: &Feed,
        sender: tokio::sync::mpsc::UnboundedSender<Entry>,
    ) -> Result<(), ()> {
        if let Ok(req_result) = reqwest::get(feed.url.as_str()).await {
            if let Ok(body) = req_result.text().await {
                if let Ok(feed) = syndication::Feed::from_str(body.as_str()) {
                    match feed {
                        syndication::Feed::Atom(atom_feed) => {
                            println!("ATOM");
                            for entry in atom_feed.entries() {
                                let parsed = Entry {
                                    title: entry.title().to_string(),
                                    // date: entry.published(),
                                    date: Utc::now(),
                                    author: "".to_string(),
                                    content: entry.content().unwrap().value().unwrap().to_string(),
                                    url: entry.content().unwrap().src().unwrap().to_string(),
                                };
                                println!("{:?}", parsed);
                                sender.send(parsed).ok();
                            }
                        }
                        syndication::Feed::RSS(rss_feed) => {
                            for entry in rss_feed.items {
                                let parsed = Entry {
                                    title: entry.title.unwrap_or("".to_string()),
                                    // date: entry.pub_date.unwrap(),
                                    // date: DateTime::<Utc>::from_str(
                                    //     entry.pub_date.unwrap().as_str(),
                                    // )
                                    // .unwrap(),
                                    date: Utc::now(),
                                    author: entry.author.unwrap_or("".to_string()),
                                    content: entry.content.unwrap_or("".to_string()),
                                    url: entry.link.unwrap_or("".to_string()),
                                };
                                println!("{}", entry.pub_date.unwrap());
                                println!("{:?}", parsed);
                                sender.send(parsed).ok();
                            }
                            println!("RSS");
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
