//! Limits.

use super::*;

/// Limits for feeds.
#[derive(Clone, Serialize, Deserialize)]
pub struct Limits {
    // /// Maximum stored feed.
    // pub max_stored: Option<usize>,
    /// Maximum iterable for each feed.
    max: Option<usize>,
    /// Oldest timestamp allowed.
    oldest_sec: Option<usize>,
}

impl Limits {
    pub fn max(&self) -> usize {
        self.max.unwrap_or(1024)
    }

    pub fn oldest(&self) -> chrono::DateTime<chrono::Utc> {
        match self.oldest_sec {
            Some(sec) => {
                chrono::offset::Utc::now()
                    - chrono::TimeDelta::seconds(sec as i64)
            }

            None => chrono::DateTime::UNIX_EPOCH,
        }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max: None,
            oldest_sec: None,
        }
    }
}
