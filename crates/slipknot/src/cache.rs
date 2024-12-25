//! Cache.

use std::future::Future;

use super::*;

/// Cache for requests.
pub struct Cache {
    cache: HashMap<String, CacheEntry>,
    duration: slipfeed::Duration,
}

impl Cache {
    pub fn new(duration: slipfeed::Duration) -> Self {
        Self {
            cache: HashMap::new(),
            duration,
        }
    }

    pub async fn get(
        &mut self,
        uri: impl AsRef<str>,
        // create: impl FnOnce() -> String,
        create: impl Future<Output = String>,
    ) -> String {
        let now = slipfeed::DateTime::now();

        // Check and use cache.
        if let Some(entry) = self.cache.get(uri.as_ref()) {
            if entry.creation.clone() + self.duration.clone() > now {
                tracing::debug!("Using entry from cache");
                return entry.entry.clone();
            }
        }

        // Create entry.
        tracing::debug!("Creating new entry for cache");
        let entry = CacheEntry {
            creation: now,
            entry: create.await,
        };
        self.cache.insert(uri.as_ref().to_string(), entry.clone());
        entry.entry
    }
}

#[derive(Clone, Debug)]
struct CacheEntry {
    creation: slipfeed::DateTime,
    entry: String,
}
