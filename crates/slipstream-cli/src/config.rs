//! Slipstream configuration.

use super::*;

/// Configuration for slipstream.
/// This is parsed from the toml slipstream configuration file.
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Update frequency.
    #[serde(default, with = "humantime_serde::option")]
    pub freq: Option<std::time::Duration>,
    /// Number of workers.
    pub workers: Option<usize>,
    /// Timezone (default UTC).
    #[serde(default, alias = "time-zone", alias = "tz")]
    pub timezone: TimeZone,
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
            workers: None,
            timezone: TimeZone::default(),
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
        updater.updater = Arc::new(RwLock::new({
            let mut updater = slipfeed::Updater::new(
                slipfeed::Duration::from_seconds(match self.freq {
                    Some(freq) => freq.as_secs(),
                    None => DEFAULT_UPDATE_SEC as u64,
                }),
                self.storage.unwrap_or(1024) as usize,
            );
            if let Some(workers) = self.workers {
                updater.set_workers(workers);
            }
            updater
        }));
        updater.entry_db = Some(Arc::new(entry_db));

        if let Some(feeds) = &self.feeds {
            let world = AggregateWorld::new();

            // Add raw feeds.
            for (name, feed_def) in feeds {
                let mut attr = slipfeed::FeedAttributes::new();
                attr.display_name = Arc::new(name.clone());
                attr.freq = Some(feed_def.options().freq());
                attr.timeout = feed_def.options().oldest();
                attr.keep_empty = feed_def.options().keep_empty();
                attr.apply_tags = feed_def.options().apply_tags();
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
                    RawFeed::MastodonStatuses {
                        mastodon,
                        feed_type,
                        token,
                    } => {
                        let feed = slipfeed::MastodonFeed::new(
                            mastodon,
                            feed_type.into(),
                            token.clone(),
                        );
                        let mut inner_updater = updater.updater.write().await;
                        let id = inner_updater.add_feed(feed, attr);
                        updater.feeds.insert(name.clone(), id);
                        updater.feeds_ids.insert(id, name.clone());
                        tracing::debug!("Added mastodon feed {}.", name);
                        world.write().await.insert(name.clone(), id, None);
                    }
                    RawFeed::MastodonUserStatuses {
                        mastodon,
                        user,
                        token,
                    } => {
                        let feed = slipfeed::MastodonFeed::new(
                            mastodon,
                            slipfeed::MastodonFeedType::UserStatuses {
                                user: user.clone(),
                                id: None,
                            },
                            token.clone(),
                        );
                        let mut inner_updater = updater.updater.write().await;
                        let id = inner_updater.add_feed(feed, attr);
                        updater.feeds.insert(name.clone(), id);
                        updater.feeds_ids.insert(id, name.clone());
                        tracing::debug!("Added mastodon feed {}.", name);
                        world.write().await.insert(name.clone(), id, None);
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
    pub fn feed(&self, feed: impl AsRef<str>) -> Option<&FeedDefinition> {
        if let Some(feeds) = self.feeds.as_ref() {
            return feeds.get(feed.as_ref());
        }
        None
    }

    /// Add a feed, directly.
    pub fn add_feed(
        &mut self,
        feed_name: impl Into<String>,
        feed_def: FeedDefinition,
    ) {
        if let Some(feeds) = &mut self.feeds {
            feeds.insert(feed_name.into(), feed_def);
        } else {
            let mut feeds = HashMap::new();
            feeds.insert(feed_name.into(), feed_def);
            self.feeds = Some(feeds);
        }
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

#[derive(Clone, Debug)]
pub struct TimeZone {
    timezone: String,
    inner: TimeZoneInner,
}

impl Default for TimeZone {
    fn default() -> Self {
        Self {
            timezone: "local".into(),
            inner: TimeZoneInner::Local,
        }
    }
}

impl Serialize for TimeZone {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // serializer.serialize_newtype_struct("timezone", &self.timezone)
        serializer.serialize_str(&self.timezone)
    }
}

impl<'de> Deserialize<'de> for TimeZone {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        let lower_text = text.trim().to_lowercase();

        if lower_text == "utc"
            || lower_text == "zulu"
            || lower_text == "universal"
        {
            return Ok(Self {
                timezone: lower_text,
                inner: TimeZoneInner::Utc,
            });
        }

        if lower_text == "local" || lower_text == "" {
            return Ok(Self {
                timezone: lower_text,
                inner: TimeZoneInner::Local,
            });
        }

        let upper_text = text.trim().to_uppercase();
        let text = format!("\"{upper_text}\"");
        let de = match toml::de::ValueDeserializer::parse(&text) {
            Ok(de) => de,
            Err(e) => {
                return Err(<D::Error as serde::de::Error>::custom(e));
            }
        };
        match chrono_tz::Tz::deserialize(de) {
            Ok(tz) => Ok(Self {
                timezone: lower_text,
                inner: TimeZoneInner::RealTimeZone(tz),
            }),
            Err(e) => Err(<D::Error as serde::de::Error>::custom(e)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum TimeZoneInner {
    Utc,
    Local,
    RealTimeZone(chrono_tz::Tz),
}

impl TimeZone {
    pub fn format(&self, dt: &slipfeed::DateTime) -> String {
        let c = dt.to_chrono().with_timezone(match &self.inner {
            TimeZoneInner::RealTimeZone(tz) => {
                return dt
                    .to_chrono()
                    .with_timezone(tz)
                    .format("%Y-%m-%d %H:%M %Z")
                    .to_string();
            }
            TimeZoneInner::Utc => return dt.to_string(),
            TimeZoneInner::Local => &chrono::Local,
        });

        return c.format("%Y-%m-%d %H:%M").to_string();
    }
}
