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
}

impl FeedOptions {
    pub fn max(&self) -> usize {
        self.max.unwrap_or(1024)
    }

    pub fn freq(&self) -> slipfeed::Duration {
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

    pub fn too_old(&self, dt: &slipfeed::DateTime) -> bool {
        slipfeed::DateTime::now() > dt.clone() + self.oldest()
    }

    fn default_freq() -> slipfeed::Duration {
        slipfeed::Duration::from_seconds(7200)
    }

    fn default_oldest() -> slipfeed::Duration {
        slipfeed::Duration::from_seconds(5040000)
    }
}

impl Default for FeedOptions {
    fn default() -> Self {
        Self {
            max: None,
            freq: None,
            oldest: None,
        }
    }
}
