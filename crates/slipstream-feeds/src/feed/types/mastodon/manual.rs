//! Mastodon manual parsing.

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MastodonStatusReponseSchema(Vec<MastodonStatusSchema>);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MastodonStatusSchema {
    id: String,
    created_at: String,
    account: MastodonAccountSchema,
    url: Option<String>,
    content: String,
    media_attachments: Vec<MastodonMediaAttachmentSchema>,
    card: Option<MastodonCardSchema>,
    tags: Vec<MastodonTagSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MastodonAccountSearchResponseSchema(Vec<MastodonAccountSchema>);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MastodonAccountSchema {
    id: String,
    username: String,
    display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MastodonMediaAttachmentSchema {
    #[serde(alias = "type")]
    attachment_type: String,
    url: String,
    preview_url: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MastodonCardSchema {
    url: String,
    title: String,
    html: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MastodonTagSchema {
    name: String,
}

/// A mastodon status feed.
#[derive(Clone, Debug)]
pub struct MastodonFeed {
    /// The url of the feed.
    instance_url: String,
    /// The feed type.
    feed_type: MastodonFeedType,
    /// The auth header token.
    token: Option<String>,
}

impl MastodonFeed {
    /// Create a new mastodon syndication.
    pub fn new(
        url: impl Into<String>,
        feed_type: MastodonFeedType,
        token: Option<String>,
    ) -> Box<Self> {
        let mut instance_url: String = url.into();
        if !instance_url.starts_with("https://") {
            instance_url = format!("https://{instance_url}");
        }
        return Box::new(Self {
            instance_url,
            feed_type,
            token,
        });
    }

    /// Grab body from endpoint.
    async fn fetch(
        client: &mut reqwest::Client,
        endpoint: &str,
    ) -> Option<String> {
        let request_builder = client.get(endpoint);
        let request = match request_builder.build() {
            Ok(request) => request,
            Err(e) => {
                tracing::error!("Unable to build request: {e}");
                return None;
            }
        };

        return match client.execute(request).await {
            Ok(resp) => match resp.text().await {
                Ok(body) => Some(body),
                Err(e) => {
                    tracing::error!("Failed to parse body: {e}");
                    return None;
                }
            },
            Err(e) => {
                tracing::error!("Failed to execute: {e}");
                return None;
            }
        };
    }

    /// Fetch account id from username.
    async fn get_account_id(
        &mut self,
        client: &mut reqwest::Client,
        username: &str,
    ) -> Option<String> {
        if let Some(body) = MastodonFeed::fetch(
            client,
            &format!(
                "{}/api/v1/accounts/search?q={}",
                &self.instance_url, username
            ),
        )
        .await
        {
            let accounts = match serde_json::from_str::<
                MastodonAccountSearchResponseSchema,
            >(&body)
            {
                Ok(data) => data,
                Err(e) => {
                    tracing::error!("Failed to parse the accounts: {e}");
                    return None;
                }
            };
            for account in accounts.0 {
                return Some(account.id);
            }
        }
        return None;
    }

    /// Parse status to entry.
    fn parse_status(
        &self,
        status: &MastodonStatusSchema,
        parse_time: &DateTime,
    ) -> Option<Entry> {
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
        builder.date(
            DateTime::try_from(&status.created_at)
                .unwrap_or_else(|_| parse_time.clone()),
        );
        if let Some(url) = &status.url {
            builder.source(url);
        }
        for attachment in &status.media_attachments {
            if attachment.attachment_type == "image" {
                content = format!(
                    "{}<br></br><img src=\"{}\" alt=\"{}\"></img>",
                    &content,
                    match &attachment.preview_url {
                        Some(url) => url,
                        None => &attachment.url,
                    },
                    match &attachment.description {
                        Some(desc) => desc,
                        None => "",
                    },
                );
            }
        }
        if let Some(card) = &status.card {
            builder.other_link(Link::new(&card.url, &card.title));
            content = format!("{}<br></br>{}", &content, &card.html);
        }
        builder.source_id(&status.id);
        builder.content(html2md::rewrite_html(&content, false));

        let mut entry = builder.build();

        for tag in &status.tags {
            entry.add_tag(&Tag::new(&tag.name));
        }

        Some(entry)
    }

    /// Parse entries from response body.
    fn parse_statuses(
        &self,
        body: &str,
        parse_time: &DateTime,
        tx: UnboundedSender<Entry>,
    ) {
        let statuses =
            match serde_json::from_str::<MastodonStatusReponseSchema>(body) {
                Ok(data) => data,
                Err(e) => {
                    tracing::error!("Failed to parse the statuses: {e}");
                    return;
                }
            };

        tracing::trace!("Parsed {:?} as mastodon", self);
        for status in statuses.0.iter() {
            if let Some(entry) = self.parse_status(status, parse_time) {
                tx.send(entry).ok();
            }
        }
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
        let mut client_builder = reqwest::ClientBuilder::new();

        // Set auth header.
        if let Some(token) = &self.token {
            let auth = reqwest::header::HeaderValue::from_str(&format!(
                "Bearer {}",
                token
            ));
            match auth {
                Ok(auth) => {
                    let mut headers = reqwest::header::HeaderMap::new();
                    headers.append(reqwest::header::AUTHORIZATION, auth);
                    client_builder = client_builder.default_headers(headers);
                }
                Err(e) => {
                    tracing::error!("Failed to set auth header: {e}");
                }
            }
        }

        let mut client = match client_builder.build() {
            Ok(client) => client,
            Err(e) => {
                tracing::warn!("Unable to build client: {e}");
                return;
            }
        };

        // Execute request and parse.
        let (tx, mut rx) = unbounded_channel();
        let feed_type = self.feed_type.clone();
        match &feed_type {
            MastodonFeedType::PublicTimeline => {
                if let Some(body) = MastodonFeed::fetch(
                    &mut client,
                    &format!("{}/api/v1/timelines/public", &self.instance_url),
                )
                .await
                {
                    self.parse_statuses(&body, &ctx.parse_time, tx);
                }
            }
            MastodonFeedType::HomeTimeline => {
                if let Some(body) = MastodonFeed::fetch(
                    &mut client,
                    &format!("{}/api/v1/timelines/home", &self.instance_url),
                )
                .await
                {
                    self.parse_statuses(&body, &ctx.parse_time, tx);
                }
            }
            MastodonFeedType::UserStatuses { user, id } => {
                let id: String = match id {
                    Some(id) => id.clone(),
                    None => {
                        match self.get_account_id(&mut client, user).await {
                            Some(id) => id,
                            None => return,
                        }
                    }
                };
                if let Some(body) = MastodonFeed::fetch(
                    &mut client,
                    &format!(
                        "{}/api/v1/accounts/{}/statuses",
                        &self.instance_url, &id
                    ),
                )
                .await
                {
                    self.parse_statuses(&body, &ctx.parse_time, tx);
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
        write!(f, "<MastodonStatus url={}>", &self.instance_url)
    }
}
