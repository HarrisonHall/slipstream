//! Cache.

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
        // Key for the cache.
        uri: impl AsRef<str>,
        // Future to create the entry if not present.
        create: impl Future<Output = String>,
        // Whether or not to write the result
        behavior: CacheBehavior,
    ) -> String {
        // If skipping the cache, just return the result.
        if let CacheBehavior::Skip = behavior {
            return create.await;
        }

        let now = slipfeed::DateTime::now();

        // Check and use cache.
        if let Some(entry) = self.cache.get(uri.as_ref()) {
            if entry.creation.clone() + self.duration.clone() > now {
                tracing::debug!("Using entry from cache.");
                return entry.entry.clone();
            }
        }

        // Create entry.
        tracing::debug!("Creating new entry for cache.");
        let entry = CacheEntry {
            creation: now,
            entry: create.await,
        };
        self.cache.insert(uri.as_ref().to_string(), entry.clone());
        entry.entry
    }
}

/// An entry in the cache.
#[derive(Clone, Debug)]
struct CacheEntry {
    creation: slipfeed::DateTime,
    entry: String,
}

/// Behavior for utilizing cache.
pub enum CacheBehavior {
    /// Use the cached data or write result to cache.
    UseOrWrite,
    /// Do not use the cache. Do not use the cached data. Do not write result
    /// to cache.
    Skip,
}

impl Default for CacheBehavior {
    fn default() -> Self {
        Self::UseOrWrite
    }
}
