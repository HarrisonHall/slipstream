//! Feed entry.

use super::*;

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
