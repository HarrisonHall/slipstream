//! Filter.

use super::*;

/// A filter is a function that takes a feed and entry and returns true if it passes, or
/// false if it fails.
pub type Filter = Arc<dyn Fn(&dyn Feed, &Entry) -> bool + Send + Sync>;
