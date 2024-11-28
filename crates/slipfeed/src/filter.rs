//! Filter.

use super::*;

/// A filter is a function that takes a feed and entry and returns true if it passes, or
/// false if it fails.
// pub type Filter = fn(&Feed, &Entry) -> bool;
pub type Filter = Arc<dyn Fn(&Feed, &Entry) -> bool + Send + Sync>;
