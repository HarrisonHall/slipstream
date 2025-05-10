//! Web module.

use core::str;

use handlebars::Handlebars;

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
    templater: Arc<handlebars::Handlebars<'static>>,
    cache: HashMap<String, CacheEntry>,
    duration: slipfeed::Duration,
}

impl HtmlServer {
    pub fn new(duration: slipfeed::Duration) -> Result<Self> {
        let mut handlebars = Handlebars::new();
        handlebars.register_template_string(
            "feed",
            (*HtmlServer::read_file("template.html")?).clone(),
        )?;
        Ok(Self {
            favicon: HtmlServer::read_file_bytes("favicon.ico")?,
            styles: HtmlServer::read_file("pico.blue.min.css")?,
            robots_txt: HtmlServer::read_file("robots.txt")?,
            cache: HashMap::new(),
            templater: Arc::new(handlebars),
            duration,
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
            Some(f) => Ok(Arc::new(Vec::from(f.data.into_owned()))),
            None => bail!("Invalid file {}.", name.as_ref()),
        }
    }

    pub async fn get(
        &mut self,
        uri: impl AsRef<str>,
        entries: impl Future<Output = Vec<slipfeed::Entry>>,
        updater: Arc<Mutex<Updater>>,
        _config: Arc<Config>,
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
            let updater = updater.lock().await;
            params = TemplateParams {
                feed: String::from(uri.as_ref()),
                entries: entries
                    .iter()
                    .map(|e| {
                        let mut sources = Vec::<String>::new();
                        let mut min = MinEntry::from(e);
                        for source in e.feeds() {
                            if let Some(source_name) =
                                updater.feed_name(*source)
                            {
                                sources.push(source_name.clone());
                            }
                        }
                        if sources.len() > 0 {
                            min.sources = sources.join(", ");
                        } else {
                            min.sources = "<Unknown Source>".into();
                        }
                        min
                    })
                    .collect(),
            };
        }
        let page = match self.templater.render("feed", &params) {
            Ok(page) => page,
            Err(e) => {
                tracing::error!("Unable to render page {}.", e);
                return "500".into();
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct TemplateParams {
    feed: String,
    entries: Vec<MinEntry>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct MinEntry {
    title: String,
    date: String,
    author: String,
    sources: String,
    source: slipfeed::Link,
    comments: slipfeed::Link,
    links: Vec<slipfeed::Link>,
}

impl From<&slipfeed::Entry> for MinEntry {
    fn from(value: &slipfeed::Entry) -> Self {
        Self {
            title: value.title().clone(),
            date: value.date().clone().pretty_string(),
            author: value.author().clone(),
            sources: String::default(),
            source: value.source().clone(),
            comments: value.comments().clone(),
            links: value.other_links().clone(),
        }
    }
}
