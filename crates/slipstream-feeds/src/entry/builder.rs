//! Feed entry.

use super::*;

/// Builder helper for entries.
#[derive(Default)]
pub struct EntryBuilder {
    title: Option<String>,
    date: Option<EntryDate>,
    author: Option<String>,
    content: Option<String>,
    source: Option<Link>,
    comments: Option<Link>,
    other_links: Vec<Link>,
    icon: Option<Link>,
    source_id: Option<String>,
}

impl EntryBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
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

    /// Set the icon link.
    pub fn icon(&mut self, url: impl Into<String>) -> &mut Self {
        self.icon = Some(Link {
            url: url.into(),
            title: "Icon".into(),
            mime_type: None,
        });
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
            title: self.title.clone().unwrap_or_default(),
            date: self
                .date
                .clone()
                .unwrap_or_else(|| EntryDate::Parsed(DateTime::now())),
            author: self.author.clone().unwrap_or_default(),
            content: self.content.clone().unwrap_or_default(),

            source: self
                .source
                .clone()
                .unwrap_or_else(|| Link::new("", "Source")),
            comments: self
                .comments
                .clone()
                .unwrap_or_else(|| Link::new("", "Comments")),
            other_links: self.other_links.clone(),
            icon: self.icon.clone(),

            source_id: self.source_id.clone(),
            primary_feed: None,
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
