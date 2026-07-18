//! Easy-to-use pre-made filters.

use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Filters {
    /// Exclude from all fields.
    #[serde(alias = "exclude", alias = "exclude-substrings")]
    pub exclude: Option<Vec<String>>,
    /// Must include from any fields.
    #[serde(alias = "include", alias = "must-include-substrings")]
    pub include: Option<Vec<String>>,
    /// Exclude from all fields (regex).
    #[serde(alias = "exclude-re")]
    pub exclude_re: Option<Vec<String>>,
    /// Must include from any fields (regex).
    #[serde(alias = "include-re")]
    pub include_re: Option<Vec<String>>,
    // / Exclude from all fields, if all present.
    #[serde(alias = "exclude-all")]
    pub exclude_all: Option<Vec<String>>,
    /// Must include from any fields, all must be present.
    #[serde(alias = "include-all", alias = "must-include-all-substrings")]
    pub include_all: Option<Vec<String>>,
    /// Exclude from title.
    #[serde(alias = "exclude-titles", alias = "exclude-title-words")]
    pub exclude_titles: Option<Vec<String>>,
    /// Must include from title.
    #[serde(alias = "include-titles")]
    pub include_titles: Option<Vec<String>>,
    /// Exclude from contents.
    #[serde(alias = "exclude-contents", alias = "exclude-content-words")]
    pub exclude_contents: Option<Vec<String>>,
    /// Must include from contents.
    #[serde(alias = "include-contents")]
    pub include_contents: Option<Vec<String>>,
    /// Exclude from links.
    #[serde(alias = "exclude-links")]
    pub exclude_links: Option<Vec<String>>,
    /// Must include from links.
    #[serde(alias = "include-links")]
    pub include_links: Option<Vec<String>>,
    /// Exclude from tags.
    #[serde(alias = "exclude-tags")]
    pub exclude_tags: Option<Vec<String>>,
    /// Must include from tags.
    #[serde(alias = "include-tags")]
    pub include_tags: Option<Vec<String>>,
    /// Exclude from tags, full tag.
    #[serde(alias = "exclude-tags-strict")]
    pub exclude_tags_strict: Option<Vec<String>>,
    /// Must include from tags, full tag.
    #[serde(alias = "include-tags-strict")]
    pub include_tags_strict: Option<Vec<String>>,
}

impl Filters {
    pub fn get_filters(&self) -> Vec<slipfeed::Filter> {
        let mut filters: Vec<slipfeed::Filter> = Vec::new();
        if let Some(filter) = exclude(&self.exclude) {
            filters.push(filter);
        }
        if let Some(filter) = include(&self.include) {
            filters.push(filter);
        }
        if let Some(filter) = exclude_re(&self.exclude_re) {
            filters.push(filter);
        }
        if let Some(filter) = include_re(&self.include_re) {
            filters.push(filter);
        }
        if let Some(filter) = exclude_all(&self.exclude_all) {
            filters.push(filter);
        }
        if let Some(filter) = include_all(&self.include_all) {
            filters.push(filter);
        }
        if let Some(filter) = exclude_titles(&self.exclude_titles) {
            filters.push(filter);
        }
        if let Some(filter) = include_titles(&self.include_titles) {
            filters.push(filter);
        }
        if let Some(filter) = exclude_contents(&self.exclude_contents) {
            filters.push(filter);
        }
        if let Some(filter) = include_contents(&self.include_contents) {
            filters.push(filter);
        }
        if let Some(filter) = exclude_links(&self.exclude_links) {
            filters.push(filter);
        }
        if let Some(filter) = include_links(&self.include_links) {
            filters.push(filter);
        }
        if let Some(filter) = exclude_tags(&self.exclude_tags) {
            filters.push(filter);
        }
        if let Some(filter) = include_tags(&self.include_tags) {
            filters.push(filter);
        }
        if let Some(filter) = exclude_tags_strict(&self.exclude_tags_strict) {
            filters.push(filter);
        }
        if let Some(filter) = include_tags_strict(&self.include_tags_strict) {
            filters.push(filter);
        }
        filters
    }
}

impl Default for Filters {
    fn default() -> Self {
        Self {
            exclude: None,
            exclude_re: None,
            exclude_all: None,
            include: None,
            include_re: None,
            include_all: None,
            exclude_titles: None,
            include_titles: None,
            exclude_contents: None,
            include_contents: None,
            exclude_links: None,
            include_links: None,
            exclude_tags: None,
            include_tags: None,
            exclude_tags_strict: None,
            include_tags_strict: None,
        }
    }
}

fn exclude_any_generic(
    items: Vec<String>,
    f: fn(&str, &slipfeed::Entry) -> bool,
) -> Option<slipfeed::Filter> {
    return Some(Arc::new(move |_feed, entry| {
        !items.iter().any(|i| f(&i, entry))
    }));
}

fn exclude_all_generic(
    items: Vec<String>,
    f: fn(&str, &slipfeed::Entry) -> bool,
) -> Option<slipfeed::Filter> {
    return Some(Arc::new(move |_feed, entry| {
        !items.iter().all(|i| f(&i, entry))
    }));
}

fn include_any_generic(
    items: Vec<String>,
    f: fn(&str, &slipfeed::Entry) -> bool,
) -> Option<slipfeed::Filter> {
    let items = items.clone();
    return Some(Arc::new(move |_feed, entry| {
        items.iter().any(|i| f(&i, entry))
    }));
}

fn include_all_generic(
    items: Vec<String>,
    f: fn(&str, &slipfeed::Entry) -> bool,
) -> Option<slipfeed::Filter> {
    let items = items.clone();
    return Some(Arc::new(move |_feed, entry| {
        items.iter().all(|i| f(&i, entry))
    }));
}

fn exclude(items: &Option<Vec<String>>) -> Option<slipfeed::Filter> {
    match &items {
        Some(items) => {
            let items: Vec<String> =
                items.clone().iter().map(|e| e.to_lowercase()).collect();
            exclude_any_generic(items, |item, entry| {
                entry.title().to_lowercase().contains(item)
                    || entry.content().to_lowercase().contains(item)
                    || entry.source().url.to_lowercase().contains(item)
            })
        }
        None => None,
    }
}

fn include(items: &Option<Vec<String>>) -> Option<slipfeed::Filter> {
    match &items {
        Some(items) => {
            let items: Vec<String> =
                items.clone().iter().map(|e| e.to_lowercase()).collect();
            include_any_generic(items, |item, entry| {
                entry.title().to_lowercase().contains(item)
                    || entry.content().to_lowercase().contains(item)
                    || entry.source().url.to_lowercase().contains(item)
            })
        }
        None => None,
    }
}

fn exclude_re(items: &Option<Vec<String>>) -> Option<slipfeed::Filter> {
    match &items {
        Some(items) => {
            let items: Vec<String> = items.clone();
            exclude_any_generic(items, |item, entry| {
                let pattern = match regex::Regex::new(item) {
                    Ok(re) => re,
                    Err(e) => {
                        tracing::warn!("Failed to compile regex ({item}): {e}");
                        return false;
                    }
                };
                pattern.is_match(&entry.title().to_lowercase())
                    || pattern.is_match(&entry.content().to_lowercase())
                    || pattern.is_match(&entry.source().url.to_lowercase())
            })
        }
        None => None,
    }
}

fn include_re(items: &Option<Vec<String>>) -> Option<slipfeed::Filter> {
    match &items {
        Some(items) => {
            let items: Vec<String> = items.clone();
            include_any_generic(items, |item, entry| {
                let pattern = match regex::Regex::new(item) {
                    Ok(re) => re,
                    Err(e) => {
                        tracing::warn!("Failed to compile regex ({item}): {e}");
                        return false;
                    }
                };
                pattern.is_match(&entry.title().to_lowercase())
                    || pattern.is_match(&entry.content().to_lowercase())
                    || pattern.is_match(&entry.source().url.to_lowercase())
            })
        }
        None => None,
    }
}

fn exclude_all(items: &Option<Vec<String>>) -> Option<slipfeed::Filter> {
    match &items {
        Some(items) => {
            let items: Vec<String> =
                items.clone().iter().map(|e| e.to_lowercase()).collect();
            exclude_all_generic(items, |item, entry| {
                entry.title().to_lowercase().contains(item)
                    || entry.content().to_lowercase().contains(item)
                    || entry.source().url.to_lowercase().contains(item)
            })
        }
        None => None,
    }
}

fn include_all(items: &Option<Vec<String>>) -> Option<slipfeed::Filter> {
    match &items {
        Some(items) => {
            let items: Vec<String> =
                items.clone().iter().map(|e| e.to_lowercase()).collect();
            include_all_generic(items, |item, entry| {
                entry.title().to_lowercase().contains(item)
                    || entry.content().to_lowercase().contains(item)
                    || entry.source().url.to_lowercase().contains(item)
            })
        }
        None => None,
    }
}

fn exclude_titles(items: &Option<Vec<String>>) -> Option<slipfeed::Filter> {
    match &items {
        Some(items) => {
            let items: Vec<String> =
                items.clone().iter().map(|e| e.to_lowercase()).collect();
            exclude_any_generic(items, |item, entry| {
                entry.title().to_lowercase().contains(item)
            })
        }
        None => None,
    }
}

fn include_titles(items: &Option<Vec<String>>) -> Option<slipfeed::Filter> {
    match &items {
        Some(items) => {
            let items: Vec<String> =
                items.clone().iter().map(|e| e.to_lowercase()).collect();
            include_any_generic(items, |item, entry| {
                entry.title().to_lowercase().contains(item)
            })
        }
        None => None,
    }
}

fn exclude_contents(items: &Option<Vec<String>>) -> Option<slipfeed::Filter> {
    match &items {
        Some(items) => {
            let items: Vec<String> =
                items.clone().iter().map(|e| e.to_lowercase()).collect();
            exclude_any_generic(items, |item, entry| {
                entry.content().to_lowercase().contains(item)
            })
        }
        None => None,
    }
}

fn include_contents(items: &Option<Vec<String>>) -> Option<slipfeed::Filter> {
    match &items {
        Some(items) => {
            let items: Vec<String> =
                items.clone().iter().map(|e| e.to_lowercase()).collect();
            include_any_generic(items, |item, entry| {
                entry.content().to_lowercase().contains(item)
            })
        }
        None => None,
    }
}

fn exclude_links(items: &Option<Vec<String>>) -> Option<slipfeed::Filter> {
    match &items {
        Some(items) => {
            let items: Vec<String> =
                items.clone().iter().map(|e| e.to_lowercase()).collect();
            exclude_any_generic(items, |item, entry| {
                entry.source().url.to_lowercase().contains(item)
                    || entry
                        .other_links()
                        .iter()
                        .any(|link| link.url.to_lowercase().contains(item))
            })
        }
        None => None,
    }
}

fn include_links(items: &Option<Vec<String>>) -> Option<slipfeed::Filter> {
    match &items {
        Some(items) => {
            let items: Vec<String> =
                items.clone().iter().map(|e| e.to_lowercase()).collect();
            include_any_generic(items, |item, entry| {
                entry.source().url.to_lowercase().contains(item)
                    || entry
                        .other_links()
                        .iter()
                        .any(|link| link.url.to_lowercase().contains(item))
            })
        }
        None => None,
    }
}

fn exclude_tags(items: &Option<Vec<String>>) -> Option<slipfeed::Filter> {
    match &items {
        Some(items) => {
            let items: Vec<String> =
                items.clone().iter().map(|e| e.to_lowercase()).collect();
            exclude_any_generic(items, |item, entry| entry.has_tag_fuzzy(item))
        }
        None => None,
    }
}

fn include_tags(items: &Option<Vec<String>>) -> Option<slipfeed::Filter> {
    match &items {
        Some(items) => {
            let items: Vec<String> =
                items.clone().iter().map(|e| e.to_lowercase()).collect();
            include_any_generic(items, |item, entry| entry.has_tag_fuzzy(item))
        }
        None => None,
    }
}

fn exclude_tags_strict(
    items: &Option<Vec<String>>,
) -> Option<slipfeed::Filter> {
    match &items {
        Some(items) => {
            let items: Vec<String> =
                items.clone().iter().map(|e| e.to_lowercase()).collect();
            exclude_any_generic(items, |item, entry| entry.has_tag(item))
        }
        None => None,
    }
}

fn include_tags_strict(
    items: &Option<Vec<String>>,
) -> Option<slipfeed::Filter> {
    match &items {
        Some(items) => {
            let items: Vec<String> =
                items.clone().iter().map(|e| e.to_lowercase()).collect();
            include_any_generic(items, |item, entry| entry.has_tag(item))
        }
        None => None,
    }
}
