//! Slipstream configuration.

use super::*;

/// Configuration for slipstream.
/// This is parsed from the toml slipstream configuration file.
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Update frequency.
    #[serde(default, with = "humantime_serde::option")]
    pub freq: Option<std::time::Duration>,
    /// Log file.
    pub log: Option<String>,
    /// Maximum entry storage size.
    pub storage: Option<u16>,
    /// Database cache file.
    pub database: Option<String>,
    /// Global configuration.
    #[serde(default)]
    pub global: GlobalConfig,
    /// Feed configuration.
    pub feeds: Option<HashMap<String, FeedDefinition>>,
    // Serve configuration.
    #[serde(default)]
    pub serve: ServeConfig,
    // Read configuration.
    #[serde(default)]
    pub read: ReadConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            freq: None,
            feeds: None,
            storage: None,
            database: None,
            global: GlobalConfig::default(),
            log: None,
            serve: ServeConfig::default(),
            read: ReadConfig::default(),
        }
    }
}

impl Config {
    /// Create a slipstream updater from the parsed configuration.
    pub async fn updater(&self) -> Result<Updater> {
        let entry_db = Database::new(match &self.database {
            Some(db) => db.as_str(),
            None => ":memory:",
        })
        .await?;
        let mut updater = Updater::default();
        updater.updater = Arc::new(RwLock::new(slipfeed::Updater::new(
            slipfeed::Duration::from_seconds(match self.freq {
                Some(freq) => freq.as_secs(),
                None => DEFAULT_UPDATE_SEC as u64,
            }),
            self.storage.unwrap_or(1024) as usize,
        )));
        updater.entry_db = Some(Arc::new(entry_db));

        if let Some(feeds) = &self.feeds {
            let world = AggregateWorld::new();

            // Add raw feeds.
            for (name, feed_def) in feeds {
                let mut attr = slipfeed::FeedAttributes::new();
                attr.display_name = Arc::new(name.clone());
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
                match feed_def.feed() {
                    RawFeed::Raw { url } => {
                        let feed = StandardFeed::new(
                            url,
                            self.global.user_agent.clone(),
                        );
                        let mut inner_updater = updater.updater.write().await;
                        let id = inner_updater.add_feed(feed, attr);
                        updater.feeds.insert(name.clone(), id);
                        updater.feeds_ids.insert(id, name.clone());
                        tracing::debug!("Added standard feed {}.", name);
                        world.write().await.insert(name.clone(), id, None);
                    }
                    RawFeed::Aggregate { feeds } => {
                        let feed = AggregateFeed::new(world.clone());
                        let mut inner_updater = updater.updater.write().await;
                        let id = inner_updater.add_feed(feed, attr);
                        updater.feeds.insert(name.clone(), id);
                        updater.feeds_ids.insert(id, name.clone());
                        tracing::debug!("Added aggregate feed {}.", name);
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

        // Add all filters.
        if let Some(all_config) = self.serve.all.as_ref() {
            updater.all_filters.extend(all_config.filters.get_filters());
        }

        Ok(updater)
    }

    /// Find a feed by name.
    pub fn feed(&self, feed: &str) -> Option<&FeedDefinition> {
        if let Some(feeds) = self.feeds.as_ref() {
            return feeds.get(feed);
        }
        None
    }
}

/// Global feed configuration.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Global filters. These apply to all feeds in the entire system.
    #[serde(default)]
    pub filters: Filters,
    /// Global options. These can be overriden by other feeds.
    #[serde(default)]
    pub limits: FeedOptions,
    /// The user agent used for StandardSyndication HTTP requests.
    /// Without specifying, no user agent is used.
    #[serde(default, alias = "user-agent")]
    pub user_agent: Option<String>,
}
