//! Config.

use super::*;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub update_delta_sec: Option<usize>,
    pub feeds: Option<HashMap<String, Feed>>,
    pub port: Option<u16>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            update_delta_sec: None,
            feeds: None,
            port: None,
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
        };
        if let Some(feeds) = &self.feeds {
            // Add raw feeds.
            for (name, feed) in feeds {
                if let Feed::Raw { url, tags, filters } = feed {
                    let id = updater.updater.add_feed(slipfeed::RawFeed {
                        url: url.clone(),
                        tags: tags
                            .clone()
                            .unwrap_or_else(|| Vec::new())
                            .iter()
                            .map(|tag| slipfeed::Tag(tag.clone()))
                            .collect(),
                        filters: filters.get_filters(),
                    });
                    updater.feeds.insert(name.clone(), id);
                    println!("Added feed {}", name);
                }
            }
            // Add aggregate feeds.
            let mut remaining_loops: u8 = 10;
            'add_loop: loop {
                if updater.feeds.len()
                    == self.feeds.iter().fold(0, |p, f| p + f.len())
                {
                    println!("Added all.");
                    break 'add_loop;
                }
                if remaining_loops == 0 {
                    println!("Feed cycles exist or a feed does not exist. Dropping remaining feeds.");
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
                    } = feed
                    {
                        let mut agg = slipfeed::AggregateFeed::new();
                        agg.tags = tags
                            .clone()
                            .unwrap_or_else(|| Vec::new())
                            .iter()
                            .map(|tag| slipfeed::Tag(tag.clone()))
                            .collect();
                        agg.filters = filters.get_filters();
                        for feed in feeds {
                            if let Some(id) = updater.feeds.get(feed) {
                                agg.feeds.push(*id);
                            } else {
                                continue 'feed_loop;
                            }
                        }
                        let id = updater.updater.add_feed(agg);
                        updater.feeds.insert(name.clone(), id);
                        println!("Added feed {}", name);
                    }
                }
                remaining_loops -= 1;
            }
        }
        updater
    }
}
