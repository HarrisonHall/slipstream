//! Config.

use slipfeed::FeedAttributes;

use super::*;

/// Configuration for slipknot.
#[derive(Serialize, Deserialize)]
pub struct Config {
    /// Update frequency.
    #[serde(default, with = "humantime_serde::option")]
    pub freq: Option<std::time::Duration>,
    /// Log file.
    pub log: Option<String>,
    /// Port.
    pub port: Option<u16>,
    /// Maximum entry storage size.
    pub storage: Option<u16>,
    /// Cache duration.
    #[serde(default, with = "humantime_serde::option")]
    pub cache: Option<std::time::Duration>,
    /// Global configuration.
    #[serde(default)]
    pub global: Global,
    /// Feed configuration.
    pub feeds: Option<HashMap<String, FeedDefinition>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            freq: None,
            feeds: None,
            port: None,
            storage: None,
            cache: None,
            global: Global::default(),
            log: None,
        }
    }
}

impl Config {
    pub async fn updater(&self) -> Updater {
        let mut updater = Updater {
            updater: slipfeed::Updater::new(
                slipfeed::Duration::from_seconds(match self.freq {
                    Some(freq) => freq.as_secs(),
                    None => DEFAULT_UPDATE_SEC as u64,
                }),
                self.storage.unwrap_or(1024) as usize,
            ),
            feeds: HashMap::new(),
            global_filters: Vec::new(),
        };

        if let Some(feeds) = &self.feeds {
            let world = AggregateWorld::new();

            // Add raw feeds.
            for (name, feed_def) in feeds {
                let mut attr = FeedAttributes::new();
                attr.freq = Some(feed_def.options().freq());
                attr.timeout = feed_def.options().oldest();
                feed_def
                    .tags()
                    .clone()
                    .unwrap_or_else(|| Vec::new())
                    .iter()
                    .for_each(|tag| attr.add_tag(tag.clone().into()));
                feed_def
                    .filters()
                    .get_filters()
                    .iter()
                    .for_each(|f| attr.add_filter(f.clone()));
                // let feed: Box<dyn slipfeed::Feed> =
                match feed_def.feed() {
                    RawFeed::Raw { url } => {
                        let feed = StandardFeed::new(url);
                        let id = updater.updater.add_feed(feed, attr);
                        updater.feeds.insert(name.clone(), id);
                        tracing::debug!("Added standard feed {}", name);
                        world.write().await.insert(name.clone(), id, None);
                    }
                    RawFeed::Aggregate { feeds } => {
                        let feed = AggregateFeed::new(world.clone());
                        let id = updater.updater.add_feed(feed, attr);
                        updater.feeds.insert(name.clone(), id);
                        tracing::debug!("Added aggregate feed {}", name);
                        world.write().await.insert(
                            name.clone(),
                            id,
                            Some(feeds.clone()),
                        );
                    }
                };
            }
        }

        // Add global filters.
        updater
            .global_filters
            .extend(self.global.filters.get_filters());

        updater
    }

    pub fn feed(&self, feed: &str) -> Option<&FeedDefinition> {
        if let Some(feeds) = self.feeds.as_ref() {
            return feeds.get(feed);
        }
        None
    }
}

/// Global configuration.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Global {
    #[serde(default)]
    pub filters: Filters,
    #[serde(default)]
    pub limits: FeedOptions,
}
