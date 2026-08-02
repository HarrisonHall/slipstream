//! Slipstream configuration.

use super::*;

const DEFAULT_FEED_STEP: u8 = 3;
const DEFAULT_FEED_AGG_STEP: u8 = 5;
const DEFAULT_FEED_TAG_STEP: u8 = 7;

/// Configuration for slipstream.
/// This is parsed from the toml slipstream configuration file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Global updater frequency.
    /// This is duration between calls to update. This is not the default feed
    /// update frequency, which is located in global limits.
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
    /// Custom commands.
    #[serde(default)]
    pub commands: Vec<CustomCommand>,
    /// Hooks.
    #[serde(default)]
    pub hooks: HashMap<Hook, Vec<Commandish>>,
    /// Feed configuration.
    pub feeds: Option<BTreeMap<String, FeedDefinition>>,
    // Serve configuration.
    #[serde(default)]
    pub serve: ServeConfig,
    // Read configuration.
    #[serde(default)]
    pub read: ReadConfig,
}

impl Config {
    /// Create a slipstream task manager from the parsed configuration.
    pub async fn build_task_manager(&self) -> Result<TaskManager> {
        let mut task_manager = TaskManager::new(Arc::new((*self).clone()));
        task_manager.updater = Arc::new(RwLock::new({
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
        task_manager.entry_db = Some(Arc::new(
            Database::new(
                match &self.database {
                    Some(db) => db.as_str(),
                    None => ":memory:",
                },
                task_manager.handle()?,
            )
            .await?,
        ));

        if let Some(feeds) = &self.feeds {
            // Add raw feeds.
            for (name, feed_def) in feeds {
                let mut attr = slipfeed::FeedAttributes::new();
                attr.display_name = Arc::new(name.clone());

                // Build options from global, overriding with feed-specific options.
                let mut options = self.global.limits.clone();
                options.merge(feed_def.options());

                attr.freq = Some(options.freq_or_default());
                attr.oldest = options.oldest();
                attr.headers = options.headers().clone();
                attr.keep_empty = options.keep_empty();
                attr.apply_tags = options.apply_tags();
                feed_def
                    .tags()
                    .clone()
                    .unwrap_or_else(Vec::new)
                    .iter()
                    .for_each(|tag| attr.add_tag(tag.clone().into()));
                feed_def
                    .filters()
                    .get_filters()
                    .iter()
                    .for_each(|f| attr.add_filter(f.clone()));
                feed_def
                    .transforms()
                    .get_transforms()
                    .iter()
                    .for_each(|f| attr.add_transform(f.clone()));

                match feed_def.feed() {
                    RawFeed::Raw { url } => {
                        attr.step = options.step(DEFAULT_FEED_STEP);
                        let feed = StandardFeed::new(url);
                        let mut inner_updater =
                            task_manager.updater.write().await;
                        let id = inner_updater.add_feed(feed, attr);
                        task_manager.feeds.insert(name.clone(), id);
                        task_manager.feeds_ids.insert(id, name.clone());
                        tracing::debug!("Added standard feed {}.", name);
                    }
                    RawFeed::Aggregate { .. } => {
                        attr.step = options.step(DEFAULT_FEED_AGG_STEP);
                        let feed = AggregateFeed::new();
                        let mut inner_updater =
                            task_manager.updater.write().await;
                        let id = inner_updater.add_feed(feed, attr);
                        task_manager.feeds.insert(name.clone(), id);
                        task_manager.feeds_ids.insert(id, name.clone());
                        tracing::debug!("Added aggregate feed {}.", name);
                    }
                    RawFeed::AggregateTag {
                        tag_allowlist,
                        tag_blocklist,
                    } => {
                        attr.step = options.step(DEFAULT_FEED_TAG_STEP);
                        let mut feed = AggregateTagFeed::new();
                        feed.allowlist = tag_allowlist
                            .iter()
                            .map(|t| slipfeed::Tag::from(t.as_str()))
                            .collect();
                        feed.blocklist = tag_blocklist
                            .iter()
                            .map(|t| slipfeed::Tag::from(t.as_str()))
                            .collect();
                        let mut inner_updater =
                            task_manager.updater.write().await;
                        let id = inner_updater.add_feed(feed, attr);
                        task_manager.feeds.insert(name.clone(), id);
                        task_manager.feeds_ids.insert(id, name.clone());
                        tracing::debug!("Added aggregate tag feed {}.", name);
                    }
                    RawFeed::MastodonStatuses {
                        mastodon,
                        feed_type,
                        token,
                    } => {
                        attr.step = options.step(DEFAULT_FEED_STEP);
                        let feed = slipfeed::MastodonFeed::new(
                            mastodon,
                            feed_type.into(),
                            token.clone(),
                        );
                        let mut inner_updater =
                            task_manager.updater.write().await;
                        let id = inner_updater.add_feed(feed, attr);
                        task_manager.feeds.insert(name.clone(), id);
                        task_manager.feeds_ids.insert(id, name.clone());
                        tracing::debug!("Added mastodon feed {}.", name);
                    }
                    RawFeed::MastodonUserStatuses {
                        mastodon,
                        user,
                        token,
                    } => {
                        attr.step = options.step(DEFAULT_FEED_STEP);
                        let feed = slipfeed::MastodonFeed::new(
                            mastodon,
                            slipfeed::MastodonFeedType::UserStatuses {
                                user: user.clone(),
                                id: None,
                            },
                            token.clone(),
                        );
                        let mut inner_updater =
                            task_manager.updater.write().await;
                        let id = inner_updater.add_feed(feed, attr);
                        task_manager.feeds.insert(name.clone(), id);
                        task_manager.feeds_ids.insert(id, name.clone());
                        tracing::debug!("Added mastodon feed {}.", name);
                    }
                };
            }

            // Add reference to child feeds for aggregate feeds.
            for (name, feed_def) in feeds {
                match feed_def.feed() {
                    RawFeed::Aggregate { feeds: input_feeds } => {
                        // Get ids of children.
                        let mut child_ids = Vec::<slipfeed::FeedId>::new();
                        for input_feed_name in input_feeds {
                            if let Some(input_feed_id) =
                                task_manager.feeds.get(input_feed_name)
                            {
                                child_ids.push(*input_feed_id);
                            } else {
                                tracing::warn!(
                                    "Aggregate feed {} referenced feed {} that does not exist.",
                                    name,
                                    input_feed_name
                                );
                            }
                        }

                        // Apply to aggregate.
                        if let Some(aggregate_feed_id) =
                            task_manager.feeds.get(name)
                        {
                            let updater = task_manager.updater.read().await;
                            if let Some(trait_feed) =
                                updater.get_feed(*aggregate_feed_id)
                            {
                                let mut trait_feed = trait_feed.write().await;
                                if let Some(agg) =
                                    trait_feed.downcast_mut::<AggregateFeed>()
                                {
                                    agg.feed_ids = child_ids;
                                } else {
                                    tracing::error!(
                                        "Aggregate feed {} is the incorrect type.",
                                        name
                                    );
                                }
                            }
                        } else {
                            tracing::error!(
                                "Cannot initialize aggregate feed {name}, does not exist."
                            );
                        }
                        tracing::debug!(
                            "Updated children for aggregate feed {}.",
                            name,
                        );
                    }
                    _ => {
                        // Do nothing for other feeds.
                    }
                }
            }
        }

        // Add global filters & transforms.
        {
            let mut inner_updater = task_manager.updater.write().await;
            self.global
                .filters
                .get_filters()
                .into_iter()
                .for_each(|f| inner_updater.add_filter(f.clone()));
            self.global
                .transforms
                .get_transforms()
                .into_iter()
                .for_each(|t| inner_updater.add_transform(t.clone()));
        }

        // Add all filters.
        if let Some(all_config) = self.serve.all.as_ref() {
            task_manager
                .all_filters
                .extend(all_config.filters.get_filters());
        }

        Ok(task_manager)
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
            let mut feeds = BTreeMap::new();
            feeds.insert(feed_name.into(), feed_def);
            self.feeds = Some(feeds);
        }
    }

    /// Map crossterm key to reader command.
    /// This prioritizes the configured key bindings, but falls back to the
    /// defaults. If a default is not preferred, the config should specific
    /// a mapping of the key to "none".
    pub fn get_key_command(&self, key: &KeyEvent) -> Commandish {
        tracing::trace!("Key press: {:?}", key);

        for (binding, command) in self.read.bindings.iter() {
            if *key == binding.into() {
                return match command {
                    Commandish::CustomCommandRef(name) => {
                        self.get_custom_command(name.as_str())
                    }
                    _ => command.clone(),
                };
            }
        }

        if *key == UPDATE {
            Commandish::Literal(ReadCommandLiteral::Update)
        } else if *key == QUIT {
            Commandish::Literal(ReadCommandLiteral::Quit)
        } else if *key == DOWN {
            Commandish::Literal(ReadCommandLiteral::Down)
        } else if *key == UP {
            Commandish::Literal(ReadCommandLiteral::Up)
        } else if *key == LEFT {
            Commandish::Literal(ReadCommandLiteral::Left)
        } else if *key == RIGHT {
            Commandish::Literal(ReadCommandLiteral::Right)
        } else if *key == PAGE_DOWN {
            Commandish::Literal(ReadCommandLiteral::PageDown)
        } else if *key == PAGE_UP {
            Commandish::Literal(ReadCommandLiteral::PageUp)
        } else if *key == TAB {
            Commandish::Literal(ReadCommandLiteral::Swap)
        } else if *key == MENU {
            Commandish::Literal(ReadCommandLiteral::Menu)
        } else if *key == COMMAND_MODE {
            Commandish::Literal(ReadCommandLiteral::CommandMode)
        } else if *key == SEARCH_MODE {
            Commandish::Literal(ReadCommandLiteral::SearchMode)
        } else {
            Commandish::Literal(ReadCommandLiteral::None)
        }
    }

    /// Get custom command associated with a command name.
    pub fn get_custom_command(&self, name: impl AsRef<str>) -> Commandish {
        for command in &self.commands {
            if *command.name == name.as_ref() {
                return command.into();
            }
        }

        tracing::warn!(
            "Failed to get custom command by name: {}",
            name.as_ref()
        );
        Commandish::Literal(ReadCommandLiteral::None)
    }
}

/// Global feed configuration.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Global filters. These apply to all feeds in the entire system.
    #[serde(default)]
    pub filters: FiltersConfig,
    /// Global options. These can be overriden by other feeds.
    #[serde(default)]
    pub limits: FeedOptions,
    /// Transform configuration.
    #[serde(default)]
    pub transforms: TransformsConfig,
    /// The user agent used for StandardSyndication HTTP requests.
    /// Without specifying, no user agent is used.
    #[serde(default, alias = "user-agent")]
    pub user_agent: Option<String>,
}

/// Timezone configuration.
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

        if lower_text == "local" || lower_text.is_empty() {
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

        c.format("%Y-%m-%d %H:%M").to_string()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Hook {
    #[serde(alias = "on-insert")]
    OnInsert,
    #[serde(alias = "on-update")]
    OnUpdate,
    #[serde(alias = "on-read")]
    OnRead,
    #[serde(alias = "on-tag")]
    OnTag,
}
