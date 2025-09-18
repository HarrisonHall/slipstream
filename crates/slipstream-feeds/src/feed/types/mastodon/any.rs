//! Any mastodon.

use super::*;

/// A mastodon feed.
#[derive(Clone, Debug)]
pub struct MastodonFeed {
    /// The url of the mastodon instance.
    instance_url: String,
    /// The feed type.
    feed_type: MastodonFeedType,
    /// The (optional) auth token.
    token: Option<String>,
}

impl MastodonFeed {
    /// Create a new mastodon syndication.
    #[allow(unused)]
    pub fn new(
        url: impl Into<String>,
        feed_type: MastodonFeedType,
        token: Option<String>,
    ) -> Box<Self> {
        return Box::new(Self {
            instance_url: url.into(),
            feed_type,
            token,
        });
    }

    /// Parse entries from response body.
    fn parse_status(
        &self,
        status: &mastodon_async::prelude::Status,
        _parse_time: &DateTime,
    ) -> Option<Entry> {
        tracing::trace!("Parsed {:?} as mastodon", self);
        let mut builder = EntryBuilder::new();

        let mut content: String = status.content.clone();

        builder.title(format!(
            "{}: \"{}\" ({})",
            &status.account.display_name,
            html2md::rewrite_html(&status.content, false)
                .chars()
                .take(40)
                .collect::<String>(),
            &status.id
        ));
        builder.author(&status.account.username);
        builder.date(DateTime::from_unix_timestamp_s(
            status.created_at.unix_timestamp() as u64,
        ));
        if let Some(url) = &status.url {
            builder.source(url.as_str());
        }
        for attachment in &status.media_attachments {
            if attachment.media_type
                == mastodon_async::prelude::MediaType::Image
            {
                content = format!(
                    "{}<br></br><img src=\"{}\" alt=\"{}\"></img>",
                    &content,
                    match &attachment.url {
                        Some(url) => url,
                        None => &attachment.preview_url,
                    },
                    match &attachment.description {
                        Some(desc) => desc,
                        None => "",
                    },
                );
            }
        }
        if let Some(card) = &status.card {
            builder.other_link(Link::new(card.url.as_str(), &card.title));
            content = format!("{}<br></br>{}", &content, &card.html);
        }
        builder.source_id(status.id.as_ref());
        builder.content(html2md::rewrite_html(&content, false));

        let mut entry = builder.build();

        for tag in &status.tags {
            entry.add_tag(&Tag::new(&tag.name));
        }

        return Some(entry);
    }
}

impl Hash for MastodonFeed {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(self.instance_url.as_bytes());
    }
}

#[feed_trait]
impl Feed for MastodonFeed {
    async fn update(&mut self, ctx: &UpdaterContext, attr: &FeedAttributes) {
        // Generate request.

        let client_data = mastodon_async::data::Data {
            base: std::borrow::Cow::from(self.instance_url.clone()),
            client_id: "".into(),
            client_secret: "".into(),
            redirect: "".into(),
            token: match &self.token {
                Some(token) => token.clone().into(),
                None => "".into(),
            },
        };

        let mast = mastodon_async::mastodon::Mastodon::from(client_data);

        // Execute request and parse.
        let (tx, mut rx) = unbounded_channel::<Entry>();
        let feed_type = self.feed_type.clone();
        match &feed_type {
            MastodonFeedType::PublicTimeline => {
                match mast.get_public_timeline(true).await {
                    Ok(timeline) => {
                        for status in &timeline {
                            if let Some(entry) =
                                self.parse_status(status, &ctx.parse_time)
                            {
                                tx.send(entry).ok();
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to get public timeline: {e}.");
                    }
                }
            }
            MastodonFeedType::HomeTimeline => {
                match mast.get_home_timeline().await {
                    Ok(page) => {
                        for status in &page.initial_items {
                            if let Some(entry) =
                                self.parse_status(status, &ctx.parse_time)
                            {
                                tx.send(entry).ok();
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to get home timeline: {e}.");
                    }
                }
            }
            MastodonFeedType::UserStatuses { user, id } => {
                let id: String = match id {
                    Some(id) => id.clone(),
                    None => 'found: {
                        if let Ok(accounts) =
                            mast.search_accounts(user, Some(1), false).await
                        {
                            for account in &accounts.initial_items {
                                tracing::info!(
                                    "Found mastodon account with name: {user}."
                                );
                                break 'found account.id.as_ref().into();
                            }
                        }
                        tracing::warn!(
                            "Failed to find account with name: {user}."
                        );
                        return;
                    }
                };
                match mast
                    .statuses(
                        &mastodon_async::prelude::AccountId::new(id),
                        Default::default(),
                    )
                    .await
                {
                    Ok(page) => {
                        for status in &page.initial_items {
                            if let Some(entry) =
                                self.parse_status(status, &ctx.parse_time)
                            {
                                tx.send(entry).ok();
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to get user timeline: {e}");
                    }
                }
            }
        }

        // Forward the matching entries.
        while let Ok(entry) = rx.try_recv() {
            let too_old =
                *entry.date() < ctx.parse_time.clone() - attr.timeout.clone();
            if too_old {
                continue;
            }

            let passes_filters = attr.passes_filters(self, &entry);
            if !passes_filters {
                continue;
            }

            ctx.sender
                .send((
                    entry.clone(),
                    FeedRef {
                        id: ctx.feed_id,
                        name: attr.display_name.clone(),
                    },
                ))
                .ok();
        }
    }
}

impl std::fmt::Display for MastodonFeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<MastodonFeed url={}>", &self.instance_url)
    }
}
