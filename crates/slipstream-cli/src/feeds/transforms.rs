//! Easy-to-use pre-made filters.

use super::*;

use slipfeed::Tag;

/// Transforms config.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransformsConfig {
    /// Derivations add tags based on the match.
    /// E.g., "hacking" = ["rust", "zig", "python"]
    /// will add the "hacking" tag to a feed with the "zig" tag.
    #[serde(alias = "tag-derivations")]
    pub tag_derivations: Option<BTreeMap<Tag, HashSet<Tag>>>,
    /// Aliases swap the tag based on the match.
    /// E.g., "hacking" = ["rust", "zig", "python"]
    /// will transform the tag "zig" into "hacking".
    #[serde(alias = "tag-aliases")]
    pub tag_aliases: Option<BTreeMap<Tag, HashSet<Tag>>>,
    /// Extract links with regex.
    #[serde(alias = "link-extractions")]
    pub link_extractions: Option<Vec<String>>,
    /// Extract links with regex.
    /// This extracts the capture group named "link".
    #[serde(alias = "builtin-link-extractions")]
    pub builtin_link_extractions: Option<bool>,
    /// Overwrite entry author according to template.
    pub author: Option<String>,
    /// Substitute content with regex.
    pub substitutions: Option<BTreeMap<String, String>>,
}

impl TransformsConfig {
    /// Get transforms from the config.
    pub fn get_transforms(&self) -> Vec<slipfeed::Transform> {
        let mut transforms: Vec<slipfeed::Transform> = Vec::new();

        if let Some(transform) = tag_derivations(&self.tag_derivations) {
            transforms.push(transform);
        }
        if let Some(transform) = tag_aliases(&self.tag_aliases) {
            transforms.push(transform);
        }
        if let Some(transform) = link_extraction(&self.link_extractions) {
            transforms.push(transform);
        }
        if let Some(transform) =
            builtin_link_extraction(&self.builtin_link_extractions)
        {
            transforms.push(transform);
        }
        if let Some(transform) = author(&self.author) {
            transforms.push(transform);
        }
        if let Some(transform) = substitute(&self.substitutions) {
            transforms.push(transform);
        }

        transforms
    }
}

fn tag_derivations(
    tag_derivations: &Option<BTreeMap<Tag, HashSet<Tag>>>,
) -> Option<slipfeed::Transform> {
    if let Some(derivations) = &tag_derivations {
        let derivations = derivations.clone();
        return Some(Arc::new(move |entry| {
            // TODO: Figure out a way to handle this without cloning tags.
            let tags = entry.tags().clone();
            for tag in tags {
                for (derivation, matches) in &derivations {
                    if matches.contains(&tag) {
                        entry.add_tag(derivation);
                    }
                }
            }
        }));
    }
    None
}

fn tag_aliases(
    tag_aliases: &Option<BTreeMap<Tag, HashSet<Tag>>>,
) -> Option<slipfeed::Transform> {
    if let Some(aliases) = &tag_aliases {
        let aliases = aliases.clone();
        return Some(Arc::new(move |entry| {
            // TODO: Figure out a way to handle this without cloning tags.
            let tags = entry.tags().clone();
            for tag in tags {
                for (alias, matches) in &aliases {
                    if matches.contains(&tag) {
                        entry.remove_tag(&tag);
                        entry.add_tag(alias);
                        return;
                    }
                }
            }
        }));
    }
    None
}

fn link_extraction(
    patterns: &Option<Vec<String>>,
) -> Option<slipfeed::Transform> {
    match &patterns {
        Some(patterns) => {
            let patterns = patterns.clone();
            Some(Arc::new(move |entry| {
                for pattern in patterns.iter() {
                    let pattern = match regex::Regex::new(pattern) {
                        Ok(re) => re,
                        Err(e) => {
                            tracing::warn!(
                                "Failed to compile regex ({pattern}): {e}"
                            );
                            continue;
                        }
                    };

                    let mut links = Vec::new();
                    for capture in pattern.captures_iter(entry.content()) {
                        if let Some(item) = capture.name("link") {
                            links.push(item.as_str().to_string());
                        }
                    }
                    for link in links {
                        entry.add_link(link, None);
                    }
                }
            }))
        }
        None => None,
    }
}

fn builtin_link_extraction(
    extract: &Option<bool>,
) -> Option<slipfeed::Transform> {
    if let Some(extract) = extract {
        if *extract == false {
            return None;
        }
        return Some(Arc::new(move |entry| {
            let finder = linkify::LinkFinder::new();
            let links: Vec<String> = finder
                .links(entry.content())
                .map(|l| l.as_str().to_string())
                .collect();
            for link in links {
                entry.add_link(link, None);
            }
        }));
    }
    None
}

fn author(template: &Option<String>) -> Option<slipfeed::Transform> {
    if let Some(template) = &template {
        let template = template.clone();
        return Some(Arc::new(move |entry| {
            let mut author = template.clone();

            if author.contains("{author}") {
                author = author.replace("{author}", entry.author());
            }
            if author.contains("{url}") {
                author = author.replace("{url}", &entry.source().url);
            }
            if author.contains("{feed}") {
                author = author.replace("{feed}", &entry.primary_feed().name);
            }
            if author.contains("{feed_fallback}") {
                if entry.author().len() == 0 {
                    author = author
                        .replace("{feed_fallback}", &entry.primary_feed().name);
                } else {
                    author = author.replace("{feed_fallback}", &entry.author());
                }
            }

            entry.set_author(author);
        }));
    }
    None
}

fn substitute(
    substitutions: &Option<BTreeMap<String, String>>,
) -> Option<slipfeed::Transform> {
    match &substitutions {
        Some(substitutions) => {
            let substitutions = substitutions.clone();
            Some(Arc::new(move |entry| {
                for (pattern, replacement) in substitutions.iter() {
                    let pattern = match regex::Regex::new(pattern) {
                        Ok(re) => re,
                        Err(e) => {
                            tracing::warn!(
                                "Failed to compile regex ({pattern}): {e}"
                            );
                            continue;
                        }
                    };

                    let title = entry.title().clone();
                    entry.set_title(pattern.replace_all(&title, replacement));
                    let content = entry.content().clone();
                    entry.set_content(
                        pattern.replace_all(&content, replacement),
                    );
                }
            }))
        }
        None => None,
    }
}
