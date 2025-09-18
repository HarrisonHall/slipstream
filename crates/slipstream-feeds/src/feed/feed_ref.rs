//! Feed ref.

use super::*;

/// Reference to a feed.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedRef {
    /// Id of the originating feed.
    pub id: FeedId,
    /// Name of the originating feed.
    pub name: Arc<String>,
}

impl PartialOrd for FeedRef {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (*self.name).partial_cmp(&(*other.name))
    }
}

impl Ord for FeedRef {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (*self.name).cmp(&(*other.name))
    }
}
