//! Feed entry.

use super::*;

/// An entry from a feed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    // Entry fields.
    title: String,
    date: EntryDate,
    author: String,
    content: String,
    source: Link,
    comments: Link,
    other_links: Vec<Link>,
    // Meta information.
    id: Option<String>,
    feeds: HashSet<FeedId>,
    tags: HashSet<Tag>,
}

impl Entry {
    /// Get entry title.
    pub fn title(&self) -> &String {
        &self.title
    }

    /// Get entry date.
    pub fn date(&self) -> &DateTime {
        match &self.date {
            EntryDate::Published(date) => date,
            EntryDate::Parsed(date) => date,
        }
    }

    /// Get entry author.
    pub fn author(&self) -> &String {
        &self.author
    }

    /// Get entry content.
    pub fn content(&self) -> &String {
        &self.content
    }

    /// Get source link.
    pub fn source(&self) -> &Link {
        &self.source
    }

    /// Get comments link.
    pub fn comments(&self) -> &Link {
        &self.comments
    }

    /// Get other links.
    pub fn other_links(&self) -> &Vec<Link> {
        &self.other_links
    }

    pub fn feeds(&self) -> &HashSet<FeedId> {
        &self.feeds
    }

    pub fn add_feed(&mut self, feed: FeedId) {
        self.feeds.insert(feed);
    }

    pub fn from_feed(&self, feed: FeedId) -> bool {
        self.feeds.contains(&feed)
    }

    pub fn tags(&self) -> &HashSet<Tag> {
        &self.tags
    }

    pub fn add_tag(&mut self, tag: &Tag) {
        self.tags.insert(tag.clone());
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Entry) -> bool {
        if let (Some(id1), Some(id2)) = (&self.id, &other.id) {
            return id1 == id2;
        }
        if self.title != other.title {
            return false;
        }
        if self.author != other.author {
            return false;
        }
        if self.content != other.content {
            return false;
        }
        if self.source != other.source {
            return false;
        }
        if self.comments != other.comments {
            return false;
        }
        if self.other_links.len() != other.other_links.len() {
            return false;
        }
        for i in 0..self.other_links.len() {
            if self.other_links[i] != other.other_links[i] {
                return false;
            }
        }
        if let EntryDate::Parsed(dt1) = &self.date {
            if let EntryDate::Parsed(dt2) = &other.date {
                if *dt1 != *dt2 {
                    return false;
                }
            }
        }
        true
    }
}

impl Eq for Entry {}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.date().partial_cmp(&other.date())
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.date().cmp(&other.date())
    }
}

impl std::hash::Hash for Entry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.title.hash(state);
        self.author.hash(state);
        self.content.hash(state);
        self.source.url.hash(state);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum EntryDate {
    Published(DateTime),
    Parsed(DateTime),
}

impl PartialOrd for EntryDate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EntryDate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (EntryDate::Published(dt1), EntryDate::Parsed(dt2)) => {
                dt1.cmp(&dt2)
            }

            (EntryDate::Parsed(dt1), EntryDate::Published(dt2)) => {
                dt1.cmp(&dt2)
            }

            (EntryDate::Parsed(dt1), EntryDate::Parsed(dt2)) => dt1.cmp(&dt2),
            (EntryDate::Published(dt1), EntryDate::Published(dt2)) => {
                dt1.cmp(&dt2)
            }
        }
    }
}

/// Builder helper for entries.
pub struct EntryBuilder {
    title: Option<String>,
    date: Option<EntryDate>,
    author: Option<String>,
    content: Option<String>,
    source: Option<Link>,
    comments: Option<Link>,
    other_links: Vec<Link>,
    id: Option<String>,
}

impl EntryBuilder {
    pub fn new() -> Self {
        Self {
            title: None,
            date: None,
            author: None,
            content: None,
            source: None,
            comments: None,
            other_links: Vec::new(),
            id: None,
        }
    }

    pub fn title(&mut self, title: impl Into<String>) -> &mut Self {
        self.title = Some(title.into());
        self
    }

    pub fn date(&mut self, date: DateTime) -> &mut Self {
        self.date = Some(EntryDate::Published(date));
        self
    }

    pub fn author(&mut self, author: impl Into<String>) -> &mut Self {
        self.author = Some(author.into());
        self
    }

    pub fn content(&mut self, content: impl Into<String>) -> &mut Self {
        self.content = Some(content.into());
        self
    }

    pub fn source(&mut self, url: impl Into<String>) -> &mut Self {
        self.source = Some(Link {
            url: url.into(),
            title: "Source".into(),
            mime_type: None,
        });
        self
    }

    pub fn comments(&mut self, url: impl Into<String>) -> &mut Self {
        self.comments = Some(Link {
            url: url.into(),
            title: "Comments".into(),
            mime_type: None,
        });
        self
    }

    pub fn other_link(&mut self, link: Link) -> &mut Self {
        self.other_links.push(link);
        self
    }

    pub fn build(&self) -> Entry {
        Entry {
            title: self.title.clone().unwrap_or_else(|| "".to_string()),
            date: self
                .date
                .clone()
                .unwrap_or_else(|| EntryDate::Parsed(DateTime::now())),
            author: self.author.clone().unwrap_or_else(|| "".to_string()),
            content: self.content.clone().unwrap_or_else(|| "".to_string()),

            source: self
                .source
                .clone()
                .unwrap_or_else(|| Link::new("", "Source")),
            comments: self
                .comments
                .clone()
                .unwrap_or_else(|| Link::new("", "Comments")),
            other_links: self.other_links.clone(),

            id: self.id.clone(),
            tags: HashSet::new(),
            feeds: HashSet::new(),
        }
    }
}

impl From<EntryBuilder> for Entry {
    fn from(value: EntryBuilder) -> Self {
        value.build()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub url: String,
    pub title: String,
    pub mime_type: Option<String>,
}

impl Link {
    pub fn new(url: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            title: title.into(),
            mime_type: None,
        }
    }

    pub fn new_with_mime(
        url: impl Into<String>,
        title: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self {
            url: url.into(),
            title: title.into(),
            mime_type: Some(mime_type.into()),
        }
    }
}
