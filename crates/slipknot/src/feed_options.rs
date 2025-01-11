//! Limits.

use super::*;

/// Limits for feeds.
#[derive(Clone, Serialize, Deserialize)]
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

    fn freq(&self) -> chrono::Duration {
        chrono::Duration::from_std(match self.freq {
            Some(freq) => freq,
            None => FeedOptions::default_freq(),
        })
        .unwrap_or_else(|_| {
            chrono::Duration::new(7200, 0).expect("7200 is a valid timedelta.")
        })
    }

    pub fn oldest(&self) -> chrono::DateTime<chrono::Utc> {
        match self.oldest {
            Some(oldest) => chrono::offset::Utc::now() - oldest,
            None => chrono::DateTime::UNIX_EPOCH,
        }
    }

    pub fn should_update(
        &self,
        last_update: &chrono::DateTime<chrono::Utc>,
    ) -> bool {
        chrono::offset::Utc::now() >= (*last_update + self.freq())
    }

    fn default_freq() -> std::time::Duration {
        std::time::Duration::from_secs(7200)
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
