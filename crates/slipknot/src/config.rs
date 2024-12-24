//! Config.

use super::*;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub update_delta_sec: Option<usize>,
    pub feeds: Option<HashMap<String, Feed>>,
    pub port: Option<u16>,
    #[serde(default)]
    pub global: Global,
    pub log: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            update_delta_sec: None,
            feeds: None,
            port: None,
            global: Global::default(),
            log: None,
        }
    }
}

impl Config {
    pub fn updater(&self) -> Updater {
        let mut updater = Updater {
            updater: slipfeed::FeedUpdater::new(Duration::seconds(
                self.update_delta_sec.unwrap_or(DEFAULT_UPDATE_SEC as usize)
                    as i64,
            )),
            feeds: HashMap::new(),
            global_filters: Vec::new(),
        };
        if let Some(feeds) = &self.feeds {
            // Add raw feeds.
            for (name, feed) in feeds {
                if let Feed::Raw {
                    url,
                    tags,
                    filters,
                    limits: _,
                } = feed
                {
                    let mut feed = slipfeed::Feed::from_raw(&url);
                    tags.clone()
                        .unwrap_or_else(|| Vec::new())
                        .iter()
                        .for_each(|tag| feed.add_tag(tag.clone().into()));
                    filters
                        .get_filters()
                        .iter()
                        .for_each(|f| feed.add_filter(f.clone()));
                    let id = updater.updater.add_feed(feed);
                    updater.feeds.insert(name.clone(), id);
                    tracing::debug!("Added feed {}", name);
                }
            }
            // Add aggregate feeds.
            let mut remaining_loops: u8 = 10;
            'add_loop: loop {
                if updater.feeds.len()
                    == self.feeds.iter().fold(0, |p, f| p + f.len())
                {
                    tracing::trace!("Added all.");
                    break 'add_loop;
                }
                if remaining_loops == 0 {
                    tracing::warn!("Feed cycles exist or a feed does not exist. Dropping remaining feeds.");
                    break 'add_loop;
                }
                'feed_loop: for (name, feed) in feeds {
                    if updater.feeds.contains_key(name) {
                        continue 'feed_loop;
                    }
                    if let Feed::Aggregate {
                        feeds,
                        tags,
                        filters,
                        limits: _,
                    } = feed
                    {
                        let mut agg_feeds: Vec<slipfeed::FeedId> = Vec::new();
                        for subfeed in feeds {
                            if let Some(id) = updater.feeds.get(subfeed) {
                                agg_feeds.push(*id);
                            } else {
                                continue 'feed_loop;
                            }
                        }
                        let mut feed =
                            slipfeed::Feed::from_aggregate(agg_feeds);
                        tags.clone()
                            .unwrap_or_else(|| Vec::new())
                            .iter()
                            .for_each(|tag| feed.add_tag(tag.clone().into()));
                        filters
                            .get_filters()
                            .iter()
                            .for_each(|f| feed.add_filter(f.clone()));
                        let id = updater.updater.add_feed(feed);
                        updater.feeds.insert(name.clone(), id);
                        tracing::debug!("Added feed {}", name);
                    }
                }
                remaining_loops -= 1;
            }
        }
        // Add global filters.
        updater
            .global_filters
            .extend(self.global.filters.get_filters());
        updater
    }

    pub fn feed(&self, feed: &str) -> Option<&Feed> {
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
    pub limits: Limits,
}
