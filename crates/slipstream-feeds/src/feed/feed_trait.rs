//! Feed trait.

use super::*;

/// What defines a feed.
#[feed_trait]
pub trait Feed: std::fmt::Debug + Send + Sync + 'static {
    /// Fetch items from the feed.
    #[allow(unused_variables)]
    async fn update(&mut self, ctx: &UpdaterContext, attr: &FeedAttributes) {}

    /// Tag fetched entry. This serves as a method for other feeds to edit and claim
    /// ownership of other entries.
    async fn tag(
        &mut self,
        entry: &mut Entry,
        feed_id: FeedId,
        attr: &FeedAttributes,
    ) {
        // By default, we only tag our own entries.
        if entry.is_from_feed(feed_id) {
            for tag in attr.get_tags() {
                entry.add_tag(&tag);
            }
        }
    }
}
