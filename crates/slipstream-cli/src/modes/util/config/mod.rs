//! Config mode.

use std::io::Write;

use super::*;

mod cli;

pub use cli::*;

pub fn config_cli(
    config_mode: ConfigMode,
    config_path: std::path::PathBuf,
) -> Result<()> {
    match config_mode {
        ConfigMode::Verify => verify_config(config_path)?,
        ConfigMode::Export {
            config_type,
            out_file,
        } => export_config(config_path, config_type, out_file)?,
        ConfigMode::Import {
            in_type,
            in_file,
            out_file,
        } => import_config(config_path, in_type, in_file, out_file)?,
    }
    Ok(())
}

fn read_config(config_path: &std::path::PathBuf) -> Result<Config> {
    // Read file.
    let config_data = match std::fs::read_to_string(&config_path) {
        Ok(data) => data,
        Err(e) => {
            bail!(
                "Unable to read data from config file {:?}: {}.",
                config_path,
                e
            );
        }
    };

    // Parse.
    match toml::from_str::<Config>(&config_data) {
        Ok(config) => {
            return Ok(config);
        }
        Err(e) => {
            bail!("Configuration file is not valid: {}.", e);
        }
    };
}

fn verify_config(config_path: std::path::PathBuf) -> Result<()> {
    match read_config(&config_path) {
        Ok(_) => {
            println!(
                "Successfully parsed config at {}.",
                config_path.to_string_lossy()
            );
            return Ok(());
        }
        Err(e) => bail!(e),
    }
}

fn export_config(
    config_path: PathBuf,
    config_destination: ConfigDestination,
    out: PathBuf,
) -> Result<()> {
    let config = read_config(&config_path)?;
    let out_data: String = match config_destination {
        ConfigDestination::Slipstream => {
            match toml::to_string_pretty(&Arc::new(config)) {
                Ok(data) => data,
                Err(e) => bail!("Cannot write toml data: {e}"),
            }
        }
        ConfigDestination::Opml => {
            let mut opml_data = opml::OPML::default();
            opml_data.version = "1.0".into();
            match &config.feeds {
                Some(feeds) => {
                    for (feed_name, feed) in feeds.iter() {
                        match feed.feed() {
                            RawFeed::Raw { url } => {
                                opml_data.add_feed(&feed_name, url);
                                if let Some(added_feed) =
                                    opml_data.body.outlines.last_mut()
                                {
                                    added_feed.r#type = Some("rss".into());
                                }
                            }
                            RawFeed::Aggregate { .. } => {
                                // Do nothing.
                            }
                            RawFeed::AggregateTag { .. } => {
                                // Do nothing.
                            }
                            RawFeed::MastodonStatuses {
                                mastodon,
                                feed_type,
                                ..
                            } => {
                                eprintln!(
                                    "Unable to export mastodon feed: {} ({:?}).",
                                    mastodon, feed_type
                                );
                            }
                            RawFeed::MastodonUserStatuses {
                                mastodon,
                                user,
                                ..
                            } => {
                                eprintln!(
                                    "Unable to export mastodon feed: {} (User {}).",
                                    mastodon, user
                                );
                            }
                        }
                    }
                }
                None => bail!("No feeds to export."),
            }

            match opml_data.to_string() {
                Ok(d) => d,
                Err(e) => bail!("Unable to export OPML: {e}."),
            }
        }
        ConfigDestination::List => {
            let mut converted_feeds: Vec<String> = vec![];

            match &config.feeds {
                Some(feeds) => {
                    for (_feed_name, feed) in feeds.iter() {
                        match feed.feed() {
                            RawFeed::Raw { url } => {
                                converted_feeds.push(url.clone());
                            }
                            RawFeed::Aggregate { .. } => {
                                // Do nothing.
                            }
                            RawFeed::AggregateTag { .. } => {
                                // Do nothing.
                            }
                            RawFeed::MastodonStatuses {
                                mastodon,
                                feed_type,
                                ..
                            } => {
                                let base = mastodon.replace("https://", "");
                                match feed_type {
                                    MastodonFeedType::PublicTimeline => {
                                        converted_feeds.push(format!(
                                            "mastodon://{base}/public/local"
                                        ));
                                    }
                                    MastodonFeedType::HomeTimeline => {
                                        converted_feeds.push(format!(
                                            "mastodon://{base}/home"
                                        ));
                                    }
                                }
                            }
                            RawFeed::MastodonUserStatuses {
                                mastodon,
                                user,
                                ..
                            } => {
                                let base = mastodon.replace("https://", "");
                                converted_feeds
                                    .push(format!("mastodon://{base}/@{user}"));
                            }
                        }
                    }
                }
                None => tracing::warn!("No feeds to export."),
            }

            converted_feeds.join("\n")
        }
    };

    let mut out_file = std::fs::File::create(&out)?;
    out_file.write_all(out_data.as_bytes())?;
    Ok(())
}

fn import_config(
    config_path: PathBuf,
    in_type: ConfigDestination,
    in_file: PathBuf,
    out: PathBuf,
) -> Result<()> {
    let mut config = read_config(&config_path)?;

    match in_type {
        ConfigDestination::Slipstream => {
            let other_config = read_config(&in_file)?;
            if let Some(feeds) = &other_config.feeds {
                for (feed_name, feed_def) in feeds.iter() {
                    config.add_feed(feed_name, feed_def.clone());
                }
            }
        }
        ConfigDestination::Opml => {
            let in_data = match std::fs::read_to_string(&in_file) {
                Ok(data) => data,
                Err(e) => {
                    bail!(
                        "Unable to read data from in file {:?}: {}.",
                        config_path,
                        e
                    );
                }
            };
            let opml_data = opml::OPML::from_str(&in_data)?;

            for outline in &opml_data.body.outlines {
                if let Some(r#type) = &outline.r#type {
                    if r#type != "rss" {
                        eprintln!(
                            "Unable to parse {:?} as valid feed.",
                            outline
                        );
                        continue;
                    }
                    if let Some(url) = &outline.xml_url {
                        config.add_feed(
                            outline.text.clone(),
                            FeedDefinition::from_feed(RawFeed::Raw {
                                url: url.clone(),
                            }),
                        );
                    } else {
                        eprintln!(
                            "Unable to parse {:?} as valid feed.",
                            outline
                        );
                        continue;
                    }
                }
            }
        }
        ConfigDestination::List => {
            let in_data = match std::fs::read_to_string(&in_file) {
                Ok(data) => data,
                Err(e) => {
                    bail!(
                        "Unable to read data from in file {:?}: {}.",
                        config_path,
                        e
                    );
                }
            };
            for line in in_data.split("\n") {
                let line = line.trim();

                // Skip whitespace.
                if line.is_empty() {
                    continue;
                }
                if line.starts_with("//") || line.starts_with("#") {
                    continue;
                }

                // Add raw feeds.
                if line.starts_with("https://") {
                    let name = line.replace("https://", "");
                    config.add_feed(
                        name,
                        FeedDefinition::from_feed(RawFeed::Raw {
                            url: line.into(),
                        }),
                    );
                    continue;
                }

                // Add mastodon feeds.
                if line.starts_with("mastodon://") {
                    let schemeless = line.replace("mastodon://", "");
                    let base: String = schemeless[..schemeless
                        .find("/")
                        .unwrap_or_else(|| schemeless.len())]
                        .into();
                    let remaining: String = schemeless[base.len()..].into();

                    // Public timeline.
                    if remaining.ends_with("/public/local") {
                        config.add_feed(
                            format!("{base}-public"),
                            FeedDefinition::from_feed(
                                RawFeed::MastodonStatuses {
                                    mastodon: base,
                                    feed_type: MastodonFeedType::PublicTimeline,
                                    token: None,
                                },
                            ),
                        );
                        continue;
                    }

                    // Home timeline.
                    if line.ends_with("/home") {
                        config.add_feed(
                            format!("{base}-home"),
                            FeedDefinition::from_feed(
                                RawFeed::MastodonStatuses {
                                    mastodon: base,
                                    feed_type: MastodonFeedType::HomeTimeline,
                                    token: None,
                                },
                            ),
                        );
                        continue;
                    }

                    // Assume user status.
                    let user: String =
                        remaining[remaining.find("@").unwrap_or(0)..].into();
                    config.add_feed(
                        format!(
                            "{}-{}",
                            base,
                            match user.strip_prefix("@") {
                                Some(user) => user,
                                None => &user,
                            }
                        ),
                        FeedDefinition::from_feed(
                            RawFeed::MastodonUserStatuses {
                                mastodon: base,
                                user,
                                token: None,
                            },
                        ),
                    );

                    continue;
                }
            }
        }
    }

    let mut out_file = std::fs::File::create(&out)?;
    out_file.write_all(toml::to_string(&config)?.as_bytes())?;
    Ok(())
}
