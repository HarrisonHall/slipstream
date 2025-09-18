//! Feed id.

use super::*;

/// Id that represents a feed.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedId(pub(crate) usize);

impl FeedId {
    /// Create a new feed id.
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}
