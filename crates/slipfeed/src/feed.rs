//! Feed management.

use super::*;

/// Id that represents a feed.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedId(pub(crate) usize);

// TODO - trait!

/// Feed.
#[derive(Clone)]
pub struct Feed {
    pub(crate) underlying: UnderlyingFeed,
    pub(crate) tags: HashSet<Tag>,
    pub(crate) filters: Vec<Filter>,
}

// TODO: Should this be a trait?
/// Any type of feed.
#[derive(Clone, Debug)]
pub(crate) enum UnderlyingFeed {
    AggregateFeed(AggregateFeed),
    RawFeed(RawFeed),
}

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

    /// Add a filter.
    pub fn add_filter(&mut self, filter: Filter) {
        self.filters.push(filter);
    }

    /// Add a tag.
    pub fn add_tag(&mut self, tag: Tag) {
        self.tags.insert(tag);
    }

    /// Get tags for a feed.
    pub fn get_tags<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Tag> + 'a> {
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
#[derive(Clone, Debug)]
pub struct RawFeed {
    pub url: String,
}

/// An aggregate feed is a collection of other feeds and filters.
#[derive(Clone, Debug)]
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
    #[allow(dead_code)]
    pub(crate) fn add_feed(&mut self, feed: FeedId) {
        self.feeds.push(feed);
    }
}
