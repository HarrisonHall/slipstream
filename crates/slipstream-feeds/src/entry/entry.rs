//! Feed entry.

use super::*;

use std::collections::BTreeSet;

/// An entry from a feed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    // Entry fields.
    /// Entry title.
    title: String,
    /// Entry publication date.
    /// If publication date is not present, this is the parsed date.
    date: EntryDate,
    /// Entry author.
    author: String,
    /// Entry content.
    content: String,
    /// Entry source link.
    source: Link,
    /// Entry comments link.
    comments: Link,
    /// Other entry links.
    other_links: Vec<Link>,
    // Meta information.
    /// The id provided by the source.
    source_id: Option<String>,
    /// List of feeds this came from.
    feeds: BTreeSet<FeedRef>,
    /// Tags applied to this entry.
    tags: BTreeSet<Tag>,
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

    /// Get the feeds.
    pub fn feeds(&self) -> &BTreeSet<FeedRef> {
        &self.feeds
    }

    /// Add a feed.
    pub fn add_feed(&mut self, feed: FeedRef) {
        self.feeds.insert(feed);
    }

    /// Check if entry is from a feed.
    pub fn is_from_feed(&self, feed: FeedId) -> bool {
        for feed_ref in self.feeds().iter() {
            if feed_ref.id == feed {
                return true;
            }
        }
        false
    }

    /// Get the tags.
    pub fn tags(&self) -> &BTreeSet<Tag> {
        &self.tags
    }

    /// Add a tag.
    pub fn add_tag(&mut self, tag: &Tag) {
        self.tags.insert(tag.clone());
    }

    /// Remove a tag.
    pub fn remove_tag(&mut self, tag: &Tag) {
        self.tags.remove(tag);
    }

    /// Check if a tag exists.
    pub fn has_tag(&self, tag: &Tag) -> bool {
        self.tags.contains(tag)
    }

    /// Check if a tag exists, fuzzily.
    pub fn has_tag_fuzzy(&self, tag: impl AsRef<str>) -> bool {
        for other_tag in &self.tags {
            if other_tag
                .to_string()
                .to_lowercase()
                .contains(&tag.as_ref().to_lowercase())
            {
                return true;
            }
        }
        return false;
    }

    /// Get the source id.
    pub fn source_id(&self) -> Option<&str> {
        match &self.source_id {
            Some(id) => Some(id.as_str()),
            None => None,
        }
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Entry) -> bool {
        if let (Some(id1), Some(id2)) = (&self.source_id, &other.source_id) {
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

impl Default for Entry {
    fn default() -> Self {
        Self {
            title: "".into(),
            date: EntryDate::Parsed(DateTime::epoch()),
            author: "".into(),
            content: "".into(),
            source: Link::new("", ""),
            comments: Link::new("", ""),
            other_links: Vec::new(),
            source_id: None,
            feeds: BTreeSet::new(),
            tags: BTreeSet::new(),
        }
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
    source_id: Option<String>,
}

impl EntryBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            title: None,
            date: None,
            author: None,
            content: None,
            source: None,
            comments: None,
            other_links: Vec::new(),
            source_id: None,
        }
    }

    /// Set the title.
    pub fn title(&mut self, title: impl Into<String>) -> &mut Self {
        self.title = Some(title.into());
        self
    }

    /// Set the date.
    pub fn date(&mut self, date: DateTime) -> &mut Self {
        self.date = Some(EntryDate::Published(date));
        self
    }

    /// Set the author.
    pub fn author(&mut self, author: impl Into<String>) -> &mut Self {
        self.author = Some(author.into());
        self
    }

    /// Set the content.
    pub fn content(&mut self, content: impl Into<String>) -> &mut Self {
        self.content = Some(content.into());
        self
    }

    /// Set the source link.
    pub fn source(&mut self, url: impl Into<String>) -> &mut Self {
        self.source = Some(Link {
            url: url.into(),
            title: "Source".into(),
            mime_type: None,
        });
        self
    }

    /// Set the comments link.
    pub fn comments(&mut self, url: impl Into<String>) -> &mut Self {
        self.comments = Some(Link {
            url: url.into(),
            title: "Comments".into(),
            mime_type: None,
        });
        self
    }

    /// Add an additional link.
    pub fn other_link(&mut self, link: Link) -> &mut Self {
        self.other_links.push(link);
        self
    }

    /// Set the source id.
    pub fn source_id(&mut self, source_id: impl Into<String>) -> &mut Self {
        self.source_id = Some(source_id.into());
        self
    }

    /// Build into an entry.
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

            source_id: self.source_id.clone(),
            feeds: BTreeSet::new(),
            tags: BTreeSet::new(),
        }
    }
}

impl From<EntryBuilder> for Entry {
    fn from(value: EntryBuilder) -> Self {
        value.build()
    }
}

/// A link to resource.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    /// The link's url.
    pub url: String,
    /// The link's title.
    pub title: String,
    /// The link's mime-type.
    pub mime_type: Option<String>,
}

impl Link {
    /// Create a new link with a title.
    pub fn new(url: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            title: title.into(),
            mime_type: None,
        }
    }

    /// Create a new link with a title and mime-type.
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
