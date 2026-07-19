//! Built-in feed types.

use super::*;

mod mastodon;
mod standard_syndication;

pub use mastodon::*;
pub use standard_syndication::*;

/// A feed that does nothing for convenience.
#[derive(Clone, Debug, Default)]
pub struct NoopFeed {
    /// Empty field necessary for downcast.
    _noop: std::marker::PhantomData<()>,
}

#[feed_trait]
impl Feed for NoopFeed {
    async fn tag(
        &mut self,
        _entry: &mut Entry,
        _feed_id: FeedId,
        _attr: &FeedAttributes,
    ) {
        // Do nothing.
    }
}
