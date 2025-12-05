//! HeaderMap extension.

use super::*;

pub trait HeaderMapExt {
    /// Create a HeaderMap with appropriate HTML headers.
    fn html_headers() -> HeaderMap;

    /// Create a HeaderMap with appropriate Atom headers.
    fn atom_headers() -> HeaderMap;

    /// Create a HeaderMap with appropriate TOML headers.
    fn toml_headers() -> HeaderMap;

    /// Create a HeaderMap with appropriate CSS headers.
    fn css_headers() -> HeaderMap;

    /// Create a HeaderMap with appropriate plaintext headers.
    fn plaintext_headers() -> HeaderMap;

    /// Create a HeaderMap with appropriate favicon headers.
    fn favicon_headers() -> HeaderMap;

    /// Grab the If-Modified-Since header as a datetime, if present.
    fn if_modified_since(&self) -> Option<slipfeed::DateTime>;

    /// Change cache behavior based on header values.
    fn cache_behavior(&self) -> crate::feeds::CacheBehavior;
}

impl HeaderMapExt for HeaderMap {
    fn html_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("text/html; charset=utf-8"),
        );
        headers
    }

    fn atom_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/atom+xml"),
        );
        headers
    }

    fn toml_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/toml"),
        );
        headers
    }
    fn css_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("text/css"),
        );
        headers
    }

    fn plaintext_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("text/plain"),
        );
        headers
    }

    fn favicon_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("image/x-icon"),
        );
        headers
    }

    fn if_modified_since(&self) -> Option<slipfeed::DateTime> {
        if let Some(header) = self.get(axum::http::header::IF_MODIFIED_SINCE) {
            if let Ok(since) = header.to_str() {
                return slipfeed::DateTime::from_if_modified_since(since);
            }
        }

        return None;
    }

    fn cache_behavior(&self) -> CacheBehavior {
        let ims = self.if_modified_since();
        match ims {
            Some(_) => CacheBehavior::Skip,
            None => CacheBehavior::UseOrWrite,
        }
    }
}
