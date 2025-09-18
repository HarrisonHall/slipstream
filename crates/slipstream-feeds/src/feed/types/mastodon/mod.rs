//! Mastodon parsing

use super::*;

mod any;
mod manual;

// pub use any::*;
pub use manual::*;

#[derive(Clone, Debug)]
pub enum MastodonFeedType {
    PublicTimeline,
    HomeTimeline,
    UserStatuses { user: String, id: Option<String> },
}
