//! Feed management.

use super::*;

/// Id that represents a feed.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedId(pub(crate) usize);

/// Attributes all feeds must have.
#[derive(Clone)]
pub struct FeedAttributes {
    /// How old entries must be, to be ignored.
    pub timeout: Duration,
    /// How often the feed should update.
    pub freq: Option<Duration>,
    /// Tags associated with the feed.
    pub tags: HashSet<Tag>,
    /// Filters for the feed.
    pub filters: Vec<Filter>,
}

impl FeedAttributes {
    /// Generate empty feed info.
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_seconds(15),
            freq: None,
            tags: HashSet::new(),
            filters: Vec::new(),
        }
    }

    /// Add a filter.
    pub fn add_filter(&mut self, filter: Filter) {
        self.filters.push(filter);
    }

    /// Add a tag.
    pub fn add_tag(&mut self, tag: Tag) {
        self.tags.insert(tag);
    }

    /// Get tags for a feed.
    pub fn get_tags<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Tag> + 'a> {
        return Box::new(self.tags.iter());
    }

    /// Check if entry passes filters.
    pub fn passes_filters(&self, feed: &dyn Feed, entry: &Entry) -> bool {
        self.filters.iter().all(|filter| filter(feed, entry))
    }
}

impl std::fmt::Debug for FeedAttributes {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        f.debug_struct("FeedInfo")
            .field("tags", &self.tags)
            .finish()
    }
}

/// What defines a feed.
#[feed_trait]
pub trait Feed: std::fmt::Debug + Send + Sync + 'static {
    /// Fetch items from the feed.
    #[allow(unused_variables)]
    async fn update(&mut self, ctx: &UpdaterContext, attr: &FeedAttributes) {}

    /// Tag fetched entry. This serves as a method for other feeds to edit and claim
    /// ownership of other entries.
    async fn tag(
        &mut self,
        entry: &mut Entry,
        feed_id: FeedId,
        attr: &FeedAttributes,
    ) {
        // By default, we only tag our own entries.
        if entry.from_feed(feed_id) {
            for tag in attr.get_tags() {
                entry.add_tag(&tag);
            }
        }
    }
}

/// A reference to an RSS/Atom feed.
#[derive(Clone, Debug)]
pub struct StandardSyndication {
    url: String,
}

impl StandardSyndication {
    pub fn new(url: impl Into<String>) -> Box<Self> {
        Box::new(Self { url: url.into() })
    }
}

#[feed_trait]
impl Feed for StandardSyndication {
    async fn update(&mut self, ctx: &UpdaterContext, attr: &FeedAttributes) {
        // Generate request.
        let client_builder = reqwest::ClientBuilder::new();
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
                last_update.if_modified_since_time(),
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
        if let Ok(req_result) = client.execute(request).await {
            if let Ok(body) = req_result.text().await {
                let body = body.as_str();
                let mut parsed = EntryBuilder::new();
                if let Ok(atom_feed) = body.parse::<atom_syndication::Feed>() {
                    for entry in atom_feed.entries() {
                        parsed
                            .title(entry.title().to_string())
                            .date(DateTime::from_chrono(
                                entry.updated().to_utc(),
                            ))
                            .author(
                                entry
                                    .authors()
                                    .iter()
                                    .fold("".to_string(), |acc, author| {
                                        format!("{} {}", acc, author.name())
                                            .to_string()
                                    })
                                    .trim(),
                            )
                            .content(entry.content().iter().fold(
                                "".to_string(),
                                |_, cont| {
                                    format!("{}", cont.value().unwrap_or(""))
                                        .to_string()
                                },
                            ));
                        for (i, link) in entry.links().iter().enumerate() {
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
                        let entry = parsed.build();
                        let too_old = *entry.date()
                            < ctx.parse_time.clone() - attr.timeout.clone();
                        if !too_old && attr.passes_filters(self, &entry) {
                            ctx.sender.send((parsed.build(), ctx.feed_id)).ok();
                        }
                        if too_old {
                            tracing::trace!(
                                "Too old! dt={:?} parse={:?} to={:?} cutoff={:?}",
                                entry.date(),
                                ctx.parse_time,
                                attr.timeout,
                                ctx.last_update
                                    .clone()
                                    .unwrap_or(DateTime::now())
                                    + attr.timeout.clone()
                            );
                        }
                    }
                    return;
                }
                if let Ok(rss_feed) = body.parse::<rss::Channel>() {
                    for entry in rss_feed.items {
                        parsed
                            .title(entry.title().unwrap_or("").to_string())
                            .date(
                                DateTime::try_from(
                                    entry.pub_date().unwrap_or(""),
                                )
                                .unwrap_or(ctx.parse_time.clone()),
                            )
                            .author(entry.author().unwrap_or("").to_string())
                            .content(entry.content().unwrap_or("").to_string());
                        if let Some(link) = entry.link() {
                            parsed.source(link);
                        }
                        if let Some(comments) = entry.comments() {
                            parsed.comments(comments);
                        }
                        let entry = parsed.build();
                        let too_old = *entry.date()
                            < ctx.parse_time.clone() - attr.timeout.clone();
                        if !too_old && attr.passes_filters(self, &entry) {
                            ctx.sender.send((parsed.build(), ctx.feed_id)).ok();
                        }
                        if too_old {
                            tracing::trace!(
                                "Too old! dt={:?} parse={:?} to={:?} cutoff={:?}",
                                entry.date(),
                                ctx.parse_time,
                                attr.timeout,
                                ctx.last_update
                                    .clone()
                                    .unwrap_or(DateTime::now())
                                    + attr.timeout.clone()
                            );
                        }
                    }
                    return;
                }
                tracing::warn!(
                    "Unable to parse feed {:?} as atom or rss:\n\t{}",
                    self,
                    body
                );
            }
        }
    }
}

impl std::fmt::Display for StandardSyndication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<StandardSyndication url={}>", &self.url)
    }
}
