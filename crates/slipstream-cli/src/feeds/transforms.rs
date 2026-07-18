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
    #[serde(default, alias = "tag-derivations")]
    pub tag_derivations: Option<BTreeMap<Tag, HashSet<Tag>>>,
    /// Aliases swap the tag based on the match.
    /// E.g., "hacking" = ["rust", "zig", "python"]
    /// will transform the tag "zig" into "hacking".
    #[serde(default, alias = "tag-aliases")]
    pub tag_aliases: Option<BTreeMap<Tag, HashSet<Tag>>>,
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
