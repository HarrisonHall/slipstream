//! Web module.

use core::str;

use super::*;

#[derive(rust_embed::Embed)]
#[folder = "src/modes/serve/web/content"]
#[exclude = "*.asesprite"]
#[exclude = "*.png"]
struct Content;

pub struct HtmlServer {
    pub favicon: Arc<Vec<u8>>,
    pub robots_txt: Arc<String>,
    pub styles: Arc<String>,
    templater: Arc<minijinja::Environment<'static>>,
    cache: HashMap<String, CacheEntry>,
    duration: slipfeed::Duration,
    error_pages: ErrorPages,
}

impl HtmlServer {
    pub fn new(duration: slipfeed::Duration) -> Result<Self> {
        let mut templater = minijinja::Environment::new();
        for template in &["template.html"] {
            if let Err(e) = templater.add_template_owned(
                *template,
                (*HtmlServer::read_file(template)?).clone(),
            ) {
                tracing::error!("Failed to add template {template}: {e}");
                bail!("Template error.");
            }
        }
        Ok(Self {
            favicon: HtmlServer::read_file_bytes("favicon.ico")?,
            styles: HtmlServer::read_file("pico.blue.min.css")?,
            robots_txt: HtmlServer::read_file("robots.txt")?,
            cache: HashMap::new(),
            templater: Arc::new(templater),
            duration,
            error_pages: ErrorPages::new(),
        })
    }

    fn read_file(name: impl AsRef<str>) -> Result<Arc<String>> {
        match Content::get(name.as_ref()) {
            Some(f) => match str::from_utf8(&f.data) {
                Ok(s) => Ok(Arc::new(String::from(s))),
                Err(_) => bail!("Invalid file {}.", name.as_ref()),
            },
            None => bail!("Invalid file {}.", name.as_ref()),
        }
    }

    fn read_file_bytes(name: impl AsRef<str>) -> Result<Arc<Vec<u8>>> {
        match Content::get(name.as_ref()) {
            Some(f) => Ok(Arc::new(f.data.into_owned())),
            None => bail!("Invalid file {}.", name.as_ref()),
        }
    }

    pub async fn get(
        &mut self,
        uri: impl AsRef<str>,
        entries: impl Future<Output = DatabaseEntryList>,
        _updater: Arc<UpdaterHandle>,
        config: Arc<Config>,
    ) -> String {
        let now = slipfeed::DateTime::now();

        // Check and use cache.
        if let Some(entry) = self.cache.get(uri.as_ref()) {
            if entry.creation.clone() + self.duration.clone() > now {
                tracing::debug!("Using entry from cache.");
                return entry.entry.clone();
            }
        }

        // Create entry.
        tracing::debug!("Creating new entry for cache.");
        let entries = entries.await;
        let params;
        {
            params = TemplateParams {
                feed: String::from(uri.as_ref()),
                entries: entries
                    .iter_entries()
                    .map(|e| {
                        let mut sources = Vec::<String>::new();
                        let mut min = MinEntry::from_entry(e, config.as_ref());
                        for source in e.feeds() {
                            sources.push((*source.name).clone());
                        }
                        if !sources.is_empty() {
                            min.sources = sources.join(", ");
                        } else {
                            min.sources = "<Unknown Source>".into();
                        }
                        min
                    })
                    .collect(),
            };
        }

        let template = match self.templater.get_template("template.html") {
            Ok(template) => template,
            Err(e) => {
                tracing::error!("Unable to render page: {}.", e);
                return self.error_pages.error_500.clone();
            }
        };
        let page: String = match template.render(&params) {
            Ok(page) => page,
            Err(e) => {
                tracing::error!("Unable to render page: {}.", e);
                return self.error_pages.error_500.clone();
            }
        };
        let entry = CacheEntry {
            creation: now,
            entry: page,
        };
        self.cache.insert(uri.as_ref().to_string(), entry.clone());
        entry.entry
    }
}

#[derive(Clone, Debug)]
struct CacheEntry {
    creation: slipfeed::DateTime,
    entry: String,
}

struct ErrorPages {
    error_500: String,
}

impl ErrorPages {
    fn new() -> Self {
        Self {
            error_500: "500".into(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct TemplateParams {
    feed: String,
    entries: Vec<MinEntry>,
}

/// Minimum view for entry to be displayed in html.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct MinEntry {
    title: String,
    date: String,
    author: String,
    sources: String,
    source: slipfeed::Link,
    content: String,
    comments: slipfeed::Link,
    links: Vec<slipfeed::Link>,
    icon: String,
    tags: Vec<String>,
}

impl MinEntry {
    fn from_entry(value: &slipfeed::Entry, config: &Config) -> Self {
        let md_parser = pulldown_cmark::Parser::new_ext(
            value.content(),
            pulldown_cmark::Options::all(),
        );
        let mut content = String::new();
        pulldown_cmark::html::push_html(&mut content, md_parser);
        Self {
            title: value.title().clone(),
            date: config.timezone.format(value.date()),
            author: value.author().clone(),
            sources: String::default(),
            source: value.source().clone(),
            content,
            comments: value.comments().clone(),
            links: value.other_links().clone(),
            icon: match value.icon() {
                Some(icon) => icon.url.clone(),
                None => String::default(),
            },
            tags: value.tags().iter().map(|t| t.to_string()).collect(),
        }
    }
}
