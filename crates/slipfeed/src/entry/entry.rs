//! Feed entry.

use super::*;

/// An entry from a feed.
#[derive(Clone, Debug)]
pub struct Entry {
    title: String,
    date: EntryDate,
    author: String,
    content: String,
    url: String,
}

impl Entry {
    /// Get entry title.
    pub fn title(&self) -> &String {
        &self.title
    }

    /// Get entry date.
    pub fn date(&self) -> &DateTime<Utc> {
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

    /// Get entry url.
    pub fn url(&self) -> &String {
        &self.url
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Entry) -> bool {
        if self.title != other.title {
            return false;
        }
        if self.author != other.author {
            return false;
        }
        if self.content != other.content {
            return false;
        }
        if self.url != other.url {
            return false;
        }
        if let EntryDate::Parsed(dt1) = self.date {
            if let EntryDate::Parsed(dt2) = other.date {
                if dt1 != dt2 {
                    return false;
                }
            }
        }
        true
    }
}

impl Eq for Entry {}

#[derive(Clone, Debug, PartialEq, Eq)]
enum EntryDate {
    Published(DateTime<Utc>),
    Parsed(DateTime<Utc>),
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
    url: Option<String>,
}

impl EntryBuilder {
    pub fn new() -> Self {
        Self {
            title: None,
            date: None,
            author: None,
            content: None,
            url: None,
        }
    }

    pub fn title(&mut self, title: impl Into<String>) -> &mut Self {
        self.title = Some(title.into());
        self
    }

    pub fn date(&mut self, date: DateTime<Utc>) -> &mut Self {
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

    pub fn url(&mut self, url: impl Into<String>) -> &mut Self {
        self.url = Some(url.into());
        self
    }

    pub fn build(&self) -> Entry {
        Entry {
            title: self.title.clone().unwrap_or_else(|| "".to_string()),
            date: self
                .date
                .clone()
                .unwrap_or_else(|| EntryDate::Parsed(Utc::now())),
            author: self.author.clone().unwrap_or_else(|| "".to_string()),
            content: self.content.clone().unwrap_or_else(|| "".to_string()),
            url: self.url.clone().unwrap_or_else(|| "".to_string()),
        }
    }
}

impl From<EntryBuilder> for Entry {
    fn from(value: EntryBuilder) -> Self {
        value.build()
    }
}
