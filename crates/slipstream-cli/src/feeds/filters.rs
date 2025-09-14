//! Easy-to-use pre-made filters.

use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Filters {
    #[serde(alias = "exclude-title-words")]
    pub exclude_title_words: Option<Vec<String>>,
    #[serde(alias = "exclude-content-words")]
    pub exclude_content_words: Option<Vec<String>>,
    #[serde(alias = "exclude-substrings")]
    pub exclude_substrings: Option<Vec<String>>,
    #[serde(alias = "must-include-substrings")]
    pub must_include_substrings: Option<Vec<String>>,
    #[serde(alias = "must-include-all-substrings")]
    pub must_include_all_substrings: Option<Vec<String>>,
    #[serde(alias = "exclude-tags")]
    pub exclude_tags: Option<Vec<String>>,
    #[serde(alias = "include-tags")]
    pub include_tags: Option<Vec<String>>,
}

impl Filters {
    pub fn get_filters(&self) -> Vec<slipfeed::Filter> {
        let mut filters: Vec<slipfeed::Filter> = Vec::new();
        if let Some(filter) = exclude_title_words(&self.exclude_title_words) {
            filters.push(filter);
        }
        if let Some(filter) = exclude_content_words(&self.exclude_content_words)
        {
            filters.push(filter);
        }
        if let Some(filter) = exclude_substrings(&self.exclude_substrings) {
            filters.push(filter);
        }
        if let Some(filter) =
            must_include_substrings(&self.must_include_substrings)
        {
            filters.push(filter);
        }
        if let Some(filter) =
            must_include_all_substrings(&self.must_include_all_substrings)
        {
            filters.push(filter);
        }
        if let Some(filter) = exclude_tags(&self.exclude_tags) {
            filters.push(filter);
        }
        if let Some(filter) = include_tags(&self.include_tags) {
            filters.push(filter);
        }
        filters
    }
}

impl Default for Filters {
    fn default() -> Self {
        Self {
            exclude_title_words: None,
            exclude_content_words: None,
            exclude_substrings: None,
            must_include_substrings: None,
            must_include_all_substrings: None,
            exclude_tags: None,
            include_tags: None,
        }
    }
}

fn exclude_title_words(
    exclusions: &Option<Vec<String>>,
) -> Option<slipfeed::Filter> {
    if let Some(exclusions) = &exclusions {
        let exclusions = exclusions.clone();
        return Some(Arc::new(move |_feed, entry| {
            for word in entry.title().split(" ") {
                let word = word.to_lowercase();
                for exclusion in exclusions.iter() {
                    let exclusion = exclusion.to_lowercase();
                    if word == exclusion {
                        return false;
                    }
                }
            }
            true
        }));
    }
    None
}

fn exclude_content_words(
    exclusions: &Option<Vec<String>>,
) -> Option<slipfeed::Filter> {
    if let Some(exclusions) = &exclusions {
        let exclusions = exclusions.clone();
        return Some(Arc::new(move |_feed, entry| {
            for word in entry.content().split(" ") {
                let word = word.to_lowercase();
                for exclusion in exclusions.iter() {
                    let exclusion = exclusion.to_lowercase();
                    if word == exclusion {
                        return false;
                    }
                }
            }
            true
        }));
    }
    None
}

fn exclude_substrings(
    exclusions: &Option<Vec<String>>,
) -> Option<slipfeed::Filter> {
    if let Some(exclusions) = &exclusions {
        let exclusions: Vec<String> =
            exclusions.iter().map(|exc| exc.to_lowercase()).collect();
        return Some(Arc::new(move |_feed, entry| {
            let title = entry.title().to_lowercase();
            let content = entry.content().to_lowercase();
            for exclusion in exclusions.iter() {
                if title.contains(exclusion) {
                    return false;
                }
                if content.contains(exclusion) {
                    return false;
                }
            }
            true
        }));
    }
    None
}

fn must_include_substrings(
    inclusions: &Option<Vec<String>>,
) -> Option<slipfeed::Filter> {
    if let Some(inclusions) = &inclusions {
        let inclusions: Vec<String> =
            inclusions.iter().map(|exc| exc.to_lowercase()).collect();
        return Some(Arc::new(move |_feed, entry| {
            let title = entry.title().to_lowercase();
            let content = entry.content().to_lowercase();
            for exclusion in inclusions.iter() {
                if title.contains(exclusion) {
                    return true;
                }
                if content.contains(exclusion) {
                    return true;
                }
            }
            false
        }));
    }
    None
}

fn must_include_all_substrings(
    inclusions: &Option<Vec<String>>,
) -> Option<slipfeed::Filter> {
    if let Some(inclusions) = &inclusions {
        let inclusions: Vec<String> =
            inclusions.iter().map(|exc| exc.to_lowercase()).collect();
        return Some(Arc::new(move |_feed, entry| {
            let title = entry.title().to_lowercase();
            let content = entry.content().to_lowercase();
            inclusions.iter().all(|exclusion| {
                if title.contains(exclusion) {
                    return true;
                }
                if content.contains(exclusion) {
                    return true;
                }
                false
            })
        }));
    }
    None
}

fn exclude_tags(exclusions: &Option<Vec<String>>) -> Option<slipfeed::Filter> {
    if let Some(exclusions) = exclusions {
        let exclusions: Vec<slipfeed::Tag> = exclusions
            .iter()
            .map(|exc| slipfeed::Tag::new(exc.to_lowercase()))
            .collect();
        return Some(Arc::new(move |_feed, entry| {
            exclusions.iter().all(|exclusion| {
                if entry.tags().contains(exclusion) {
                    return false;
                }
                true
            })
        }));
    }
    None
}

fn include_tags(inclusions: &Option<Vec<String>>) -> Option<slipfeed::Filter> {
    if let Some(inclusions) = &inclusions {
        let inclusions: Vec<slipfeed::Tag> = inclusions
            .iter()
            .map(|exc| slipfeed::Tag::new(exc.to_lowercase()))
            .collect();
        return Some(Arc::new(move |_feed, entry| {
            inclusions.iter().any(|inclusion| {
                if entry.tags().contains(inclusion) {
                    return true;
                }
                false
            })
        }));
    }
    None
}
