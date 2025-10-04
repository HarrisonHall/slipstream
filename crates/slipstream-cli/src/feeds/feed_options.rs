//! Limits.

use super::*;

/// Limits for feeds.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeedOptions {
    /// Maximum iterable feeds from feed.
    max: Option<usize>,
    /// Update frequency. Defaults to 2 hours.
    #[serde(default, with = "humantime_serde::option")]
    freq: Option<std::time::Duration>,
    /// Oldest duration.
    #[serde(default, with = "humantime_serde::option")]
    oldest: Option<std::time::Duration>,
    /// Whether to keep empty entries (no title).
    #[serde(default = "FeedOptions::default_keep_empty", alias = "keep-empty")]
    keep_empty: bool,
    /// Whether to apply tags from the source.
    #[serde(default = "FeedOptions::default_apply_tags", alias = "apply-tags")]
    apply_tags: bool,
}

impl FeedOptions {
    pub fn max(&self) -> usize {
        self.max.unwrap_or(1024)
    }

    pub fn freq(&self) -> Option<slipfeed::Duration> {
        match self.freq {
            Some(freq) => Some(slipfeed::Duration::from_std(freq)),
            None => None,
        }
    }

    pub fn freq_or_default(&self) -> slipfeed::Duration {
        match self.freq {
            Some(freq) => slipfeed::Duration::from_std(freq),
            None => FeedOptions::default_freq(),
        }
    }

    pub fn oldest(&self) -> slipfeed::Duration {
        match self.oldest {
            Some(oldest) => slipfeed::Duration::from_std(oldest),
            None => FeedOptions::default_oldest(),
        }
    }

    pub fn keep_empty(&self) -> bool {
        self.keep_empty
    }

    pub fn apply_tags(&self) -> bool {
        self.apply_tags
    }

    pub fn too_old(&self, dt: &slipfeed::DateTime) -> bool {
        slipfeed::DateTime::now() > dt.clone() + self.oldest()
    }

    fn default_freq() -> slipfeed::Duration {
        slipfeed::Duration::from_seconds(7200)
    }

    fn default_oldest() -> slipfeed::Duration {
        slipfeed::Duration::from_seconds(5040000)
    }

    fn default_keep_empty() -> bool {
        false
    }

    fn default_apply_tags() -> bool {
        true
    }
}

impl Default for FeedOptions {
    fn default() -> Self {
        Self {
            max: None,
            freq: None,
            oldest: None,
            keep_empty: Self::default_keep_empty(),
            apply_tags: Self::default_apply_tags(),
        }
    }
}
