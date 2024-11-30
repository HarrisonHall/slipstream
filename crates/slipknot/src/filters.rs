//! Easy-to-use pre-made filters.

use super::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct Filters {
    #[serde(alias = "exclude-title-words")]
    pub exclude_title_words: Option<Vec<String>>,
    #[serde(alias = "exclude-content-words")]
    pub exclude_content_words: Option<Vec<String>>,
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
        filters
    }
}

fn exclude_title_words(
    exclusions: &Option<Vec<String>>,
) -> Option<slipfeed::Filter> {
    if let Some(exclusions) = &exclusions {
        let exclusions = Arc::new(exclusions.clone());
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
        let exclusions = Arc::new(exclusions.clone());
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
