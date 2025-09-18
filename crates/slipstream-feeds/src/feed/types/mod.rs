//! Built-in feed types.

use super::*;

mod mastodon;
mod standard_syndication;

pub use mastodon::*;
pub use standard_syndication::*;
