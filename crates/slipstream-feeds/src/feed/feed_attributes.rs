//! Feed attributes.

use super::*;

/// Attributes all feeds must have.
#[derive(Clone)]
pub struct FeedAttributes {
    /// Feed name.
    /// This need not unique-- just something consistent that can be displayed.
    pub display_name: Arc<String>,
    /// How old entries must be, to be ignored.
    pub timeout: Duration,
    /// How often the feed should update.
    pub freq: Option<Duration>,
    /// Tags associated with the feed.
    pub tags: HashSet<Tag>,
    /// Filters for the feed.
    pub filters: Vec<Filter>,
    /// Whether to keep empty entries (no title).
    pub keep_empty: bool,
    /// Whether to apply tags from the source.
    pub apply_tags: bool,
}

impl FeedAttributes {
    /// Generate empty feed info.
    pub fn new() -> Self {
        Self {
            display_name: Arc::new(":empty:".into()),
            timeout: Duration::from_seconds(15),
            freq: None,
            tags: HashSet::new(),
            filters: Vec::new(),
            keep_empty: false,
            apply_tags: true,
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
    pub fn get_tags<'a>(
        &'a self,
    ) -> std::collections::hash_set::Iter<'a, tag::Tag> {
        return self.tags.iter();
    }

    /// Check if entry passes filters.
    pub fn passes_filters(&self, feed: &dyn Feed, entry: &Entry) -> bool {
        self.filters.iter().all(|filter| filter(feed, entry))
    }
}

impl std::fmt::Debug for FeedAttributes {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        f.debug_struct("FeedInfo")
            .field("tags", &self.tags)
            .finish()
    }
}
