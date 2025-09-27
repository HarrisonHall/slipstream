//! Standard syndication (atom + rss).

use super::*;

/// A reference to an RSS/Atom feed.
#[derive(Clone, Debug)]
pub struct StandardSyndication {
    /// The url of the feed.
    url: String,
    /// The user agent used to fetch the feed.
    /// If this is not set, no user agent is used.
    user_agent: Option<String>,
}

impl StandardSyndication {
    /// Create a new standard syndication.
    pub fn new(
        url: impl Into<String>,
        user_agent: Option<String>,
    ) -> Box<Self> {
        return Box::new(Self {
            url: url.into(),
            user_agent,
        });
    }

    /// Parse a feed from the body text.
    fn parse(
        &self,
        body: &str,
        ctx: &UpdaterContext,
        attr: &FeedAttributes,
        tx: UnboundedSender<Entry>,
    ) {
        let mut parse_error = String::new();

        // Try to parse as atom.
        match body.parse::<atom_syndication::Feed>() {
            Ok(atom_feed) => {
                tracing::trace!("Parsed {:?} as atom", self);
                for atom_entry in atom_feed.entries() {
                    let entry =
                        StandardSyndication::parse_atom(atom_entry, ctx, attr);
                    if !attr.keep_empty && entry.title().is_empty() {
                        continue;
                    }
                    tx.send(entry).ok();
                }
                return;
            }
            Err(e) => {
                parse_error.push_str(&format!("\n{}", e));
            }
        }

        // Try to parse as rss.
        match body.parse::<rss::Channel>() {
            Ok(rss_feed) => {
                tracing::trace!("Parsed {:?} as rss", self);
                for rss_entry in rss_feed.items() {
                    let entry =
                        StandardSyndication::parse_rss(rss_entry, ctx, attr);
                    if !attr.keep_empty && entry.title().is_empty() {
                        continue;
                    }
                    tx.send(entry).ok();
                }
                return;
            }
            Err(e) => {
                parse_error.push_str(&format!("\n{}", e));
            }
        }

        tracing::warn!(
            "Unable to parse feed `{:?}` as atom or rss:\n\t{}\nReasons:{}",
            self,
            body,
            &parse_error
        );
    }

    /// Parse an atom entry.
    fn parse_atom(
        atom_entry: &atom_syndication::Entry,
        _ctx: &UpdaterContext,
        attr: &FeedAttributes,
    ) -> Entry {
        let mut parsed = EntryBuilder::new();
        parsed
            .title(atom_entry.title().to_string())
            .date(DateTime::from_chrono(atom_entry.updated().to_utc()))
            .author(
                atom_entry
                    .authors()
                    .iter()
                    .fold("".to_string(), |acc, author| {
                        format!("{} {}", acc, author.name()).to_string()
                    })
                    .trim(),
            )
            .content(match atom_entry.summary() {
                Some(sum) => html2md::rewrite_html(&sum.value, false),
                None => match atom_entry.content() {
                    Some(content) => html2md::rewrite_html(
                        content.value().unwrap_or(""),
                        false,
                    ),
                    None => "".into(),
                },
            });
        for (i, link) in atom_entry.links().iter().enumerate() {
            if i == 0 {
                parsed.source(&link.href);
            } else {
                parsed.other_link(Link::new_with_mime(
                    &link.href,
                    link.title().unwrap_or(""),
                    link.mime_type().unwrap_or(""),
                ));
            }
        }

        let mut entry = parsed.build();

        if attr.apply_tags {
            for category in atom_entry.categories() {
                entry.add_tag(&Tag::new(String::from(category.term.clone())));
            }
        }

        return entry;
    }

    /// Parse an rss entry.
    fn parse_rss(
        rss_entry: &rss::Item,
        ctx: &UpdaterContext,
        attr: &FeedAttributes,
    ) -> Entry {
        let mut parsed = EntryBuilder::new();
        parsed
            .title(rss_entry.title().unwrap_or(""))
            .date('date: {
                if let Ok(dt) =
                    DateTime::try_from(rss_entry.pub_date().unwrap_or(""))
                {
                    break 'date dt;
                }
                if let Some(dc) = rss_entry.dublin_core_ext() {
                    for date in dc.dates() {
                        if let Ok(dt) = DateTime::try_from(date) {
                            break 'date dt;
                        }
                    }
                }

                ctx.parse_time.clone()
            })
            .author(rss_entry.author().unwrap_or(""))
            .content(html2md::rewrite_html(
                match rss_entry.description() {
                    Some(desc) => desc,
                    None => rss_entry.content().unwrap_or(""),
                },
                false,
            ));
        if let Some(link) = rss_entry.link() {
            parsed.source(link);
        }
        if let Some(comments) = rss_entry.comments() {
            parsed.comments(comments);
        }
        let mut entry = parsed.build();
        if attr.apply_tags {
            for category in rss_entry.categories() {
                entry.add_tag(&Tag::new(category.name()));
            }
            if let Some(dc) = rss_entry.dublin_core_ext() {
                for subject in dc.subjects() {
                    entry.add_tag(&Tag::new(subject));
                }
            }
        }
        return entry;
    }
}

impl Hash for StandardSyndication {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(self.url.as_bytes());
    }
}

#[feed_trait]
impl Feed for StandardSyndication {
    async fn update(&mut self, ctx: &UpdaterContext, attr: &FeedAttributes) {
        // Generate request.
        let mut client_builder = reqwest::ClientBuilder::new();

        if let Some(user_agent) = &self.user_agent {
            client_builder = client_builder.user_agent(user_agent);
        }

        let client = match client_builder.build() {
            Ok(client) => client,
            Err(e) => {
                tracing::warn!("Unable to build client: {e}");
                return;
            }
        };
        let mut request_builder = client.get(&self.url);
        if let Some(last_update) = ctx.last_update.as_ref() {
            request_builder = request_builder.header(
                reqwest::header::IF_MODIFIED_SINCE,
                last_update.to_if_modified_since(),
            );
        };
        let request = match request_builder.build() {
            Ok(request) => request,
            Err(e) => {
                tracing::warn!("Unable to build request: {e}");
                return;
            }
        };

        // Execute request and parse.
        let (tx, mut rx) = unbounded_channel();
        match client.execute(request).await {
            Ok(req_result) => match req_result.text().await {
                Ok(body) => {
                    self.parse(body.as_str(), &ctx, attr, tx);
                }
                Err(e) => {
                    tracing::error!("Failed to get body from response: {e}")
                }
            },
            Err(e) => tracing::error!("Failed to execute: {e}"),
        };

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

impl std::fmt::Display for StandardSyndication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<StandardSyndication url={}>", &self.url)
    }
}
