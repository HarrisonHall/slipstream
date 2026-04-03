//! Limits.

use super::*;

/// Limits for feeds.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeedOptions {
    /// Maximum iterable entries from feed.
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
    /// Custom headers for this feed.
    #[serde(default)]
    headers: BTreeMap<String, String>,
    /// Feed update step (lower updates first).
    #[serde(default)]
    step: Option<usize>,
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

    pub fn step(&self, default: u8) -> u8 {
        self.step.unwrap_or(default as usize) as u8
    }

    pub fn keep_empty(&self) -> bool {
        self.keep_empty
    }

    pub fn apply_tags(&self) -> bool {
        self.apply_tags
    }

    pub fn headers(&self) -> &BTreeMap<String, String> {
        &self.headers
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

    pub(crate) fn merge(&mut self, other: &Self) {
        if let Some(max) = &other.max {
            self.max = Some(max.clone());
        }
        if let Some(freq) = &other.freq {
            self.freq = Some(freq.clone());
        }
        if let Some(oldest) = &other.oldest {
            self.oldest = Some(oldest.clone());
        }
        if let Some(step) = &other.step {
            self.step = Some(step.clone());
        }
        self.keep_empty = other.keep_empty;
        self.apply_tags = other.apply_tags;
        for (header, value) in &other.headers {
            self.headers.insert(header.clone(), value.clone());
        }
    }
}

impl Default for FeedOptions {
    fn default() -> Self {
        Self {
            max: None,
            freq: None,
            oldest: None,
            step: None,
            keep_empty: Self::default_keep_empty(),
            apply_tags: Self::default_apply_tags(),
            headers: BTreeMap::new(),
        }
    }
}
