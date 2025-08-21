//! Slipstream configuration.

use slipfeed::FeedAttributes;

use super::*;

/// Configuration for slipstream.
/// This is parsed from the toml slipstream configuration file.
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
    pub global: GlobalConfig,
    /// All configuration.
    pub all: Option<GlobalConfig>,
    /// Feed configuration.
    pub feeds: Option<HashMap<String, FeedDefinition>>,
    // Additional configuration.
    /// Put source into served title.
    #[serde(default = "Config::default_show_source_in_title")]
    pub show_source_in_title: bool,
    /// Location for archives.
    pub archive_path: Option<String>,
    // Read configuration.
    #[serde(default)]
    pub read: ReadConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            freq: None,
            feeds: None,
            port: None,
            storage: None,
            cache: None,
            global: GlobalConfig::default(),
            all: None,
            log: None,
            show_source_in_title: true,
            archive_path: None,
            read: ReadConfig::default(),
        }
    }
}

impl Config {
    /// Create a slipstream updater from the parsed configuration.
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
            feeds_ids: HashMap::new(),
            global_filters: Vec::new(),
            all_filters: Vec::new(),
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
                match feed_def.feed() {
                    RawFeed::Raw { url } => {
                        let feed = StandardFeed::new(url);
                        let id = updater.updater.add_feed(feed, attr);
                        updater.feeds.insert(name.clone(), id);
                        updater.feeds_ids.insert(id, name.clone());
                        tracing::debug!("Added standard feed {}.", name);
                        world.write().await.insert(name.clone(), id, None);
                    }
                    RawFeed::Aggregate { feeds } => {
                        let feed = AggregateFeed::new(world.clone());
                        let id = updater.updater.add_feed(feed, attr);
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
        if let Some(all_config) = self.all.as_ref() {
            updater.all_filters.extend(all_config.filters.get_filters());
        }

        updater
    }

    /// Find a feed by name.
    pub fn feed(&self, feed: &str) -> Option<&FeedDefinition> {
        if let Some(feeds) = self.feeds.as_ref() {
            return feeds.get(feed);
        }
        None
    }

    fn default_show_source_in_title() -> bool {
        false
    }
}

/// Global configuration.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(default)]
    pub filters: Filters,
    #[serde(default)]
    pub limits: FeedOptions,
}
