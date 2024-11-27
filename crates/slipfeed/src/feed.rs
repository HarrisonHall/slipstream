//! Feed management.

use std::collections::HashSet;
use std::sync::Arc;

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
pub struct FeedId(pub(crate) usize);

pub struct Feed {
    pub(crate) underlying: UnderlyingFeed,
    pub(crate) tags: HashSet<Tag>,
    pub(crate) filters: Vec<Filter>,
}

// TODO: Should this be a trait?
/// Any type of feed.
#[derive(Clone)]
pub(crate) enum UnderlyingFeed {
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
