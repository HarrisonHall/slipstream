//! Serve mode.

use super::*;

use axum::extract::State;
use axum::http::HeaderMap;

mod config;
mod header_map_ext;
mod web;

pub use config::*;
use header_map_ext::HeaderMapExt;
use web::*;

/// Serve slipstream over http.
pub async fn serve_cli(
    port: Option<u16>,
    config: Arc<Config>,
    updater: UpdaterHandle,
    cancel_token: CancellationToken,
) -> Result<()> {
    // Create caches.
    let duration = slipfeed::Duration::from_seconds(match config.serve.cache {
        Some(freq) => freq.as_secs(),
        None => 120,
    });
    let cache = Arc::new(Mutex::new(Cache::new(duration.clone())));
    let html = Arc::new(Mutex::new(HtmlServer::new(duration)?));

    // Create server.
    let app = axum::Router::new()
        .route("/", axum::routing::get(get_all_web))
        .route("/config", axum::routing::get(get_config))
        .route("/all", axum::routing::get(get_all_web))
        .route("/all/feed", axum::routing::get(get_all_atom))
        .route("/all/feed.atom", axum::routing::get(get_all_atom))
        .route("/all/feed.xml", axum::routing::get(get_all_atom))
        .route("/feed/{feed}", axum::routing::get(get_feed_web))
        .route("/feed/{feed}/feed", axum::routing::get(get_feed_atom))
        .route("/feed/{feed}/feed.atom", axum::routing::get(get_feed_atom))
        .route("/feed/{feed}/feed.xml", axum::routing::get(get_feed_atom))
        .route("/tag/{tag}", axum::routing::get(get_tag_web))
        .route("/tag/{tag}/feed", axum::routing::get(get_tag_atom))
        .route("/tag/{tag}/feed.atom", axum::routing::get(get_tag_atom))
        .route("/tag/{tag}/feed.xml", axum::routing::get(get_tag_atom))
        .route("/styles.css", axum::routing::get(get_styles))
        .route("/robots.txt", axum::routing::get(get_robots_txt))
        .route("/favicon.ico", axum::routing::get(get_favicon))
        .with_state(Arc::new(SFState {
            updater: Arc::new(updater),
            config: config.clone(),
            cache,
            html,
        }));
    let port = port.unwrap_or(config.serve.port.unwrap_or(DEFAULT_PORT));
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect(&format!("Unable to bind to port {}", port));

    // Serve.
    tracing::info!("slipstream serve");
    tracing::info!("Serving feeds @ 0.0.0.0:{}", port);

    let served = axum::serve(listener, app);
    let cancelled = cancel_token.cancelled();
    tokio::select! {
        served_res = served => {
            if let Err(e) = served_res {
                tracing::error!("Error serving: {}", e);
                cancel_token.cancel();
            }
        },
        _ = cancelled => {
            // Quit.
        },
    };

    Ok(())
}

/// State shared by the axum web server.
#[derive(Clone)]
struct SFState {
    updater: Arc<UpdaterHandle>,
    config: Arc<Config>,
    cache: Arc<Mutex<Cache>>,
    html: Arc<Mutex<HtmlServer>>,
}

/// The wrapped state type.
type StateType = axum::extract::State<Arc<SFState>>;

/// Get the web view for the /all feed.
async fn get_all_web(
    State(state): StateType,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    tracing::debug!("/all");
    let mut html = state.html.lock().await;
    let updater = state.updater.clone();
    return (
        HeaderMap::html_headers(),
        html.get(
            "/all",
            async move { updater.collect_all(headers.if_modified_since()).await },
            state.updater.clone(),
            state.config.clone(),
        )
        .await,
    );
}

/// Get the atom feed for the /all feed.
async fn get_all_atom(
    State(state): StateType,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    tracing::debug!("/all/feed");
    let config = state.config.clone();
    let updater = state.updater.clone();
    let mut cache = state.cache.lock().await;
    let cache_behavior = headers.cache_behavior();
    return (
        HeaderMap::atom_headers(),
        cache
            .get(
                "/all",
                async move {
                    updater
                        .syndicate_all(config, headers.if_modified_since())
                        .await
                },
                cache_behavior,
            )
            .await,
    );
}

/// Get the web view for a feed.
async fn get_feed_web(
    State(state): StateType,
    headers: HeaderMap,
    uri: axum::http::Uri,
) -> impl axum::response::IntoResponse {
    tracing::debug!("{}", uri.path());
    let feed = &uri.path()["/feed/".len()..];
    let updater = state.updater.clone();
    let mut html = state.html.lock().await;
    return (
        HeaderMap::html_headers(),
        html.get(
            uri.path(),
            async move {
                updater
                    .collect_feed(feed, headers.if_modified_since())
                    .await
            },
            state.updater.clone(),
            state.config.clone(),
        )
        .await,
    );
}

/// Get the atom feed for a feed.
async fn get_feed_atom(
    State(state): StateType,
    headers: HeaderMap,
    uri: axum::http::Uri,
    axum::extract::Path(feed): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    tracing::debug!("{}", uri.path());
    let config = state.config.clone();
    let updater = state.updater.clone();
    let mut cache = state.cache.lock().await;
    let cache_behavior = headers.cache_behavior();
    return (
        HeaderMap::atom_headers(),
        cache
            .get(
                uri.path(),
                async move {
                    updater
                        .syndicate_feed(
                            &feed,
                            config,
                            headers.if_modified_since(),
                        )
                        .await
                },
                cache_behavior,
            )
            .await,
    );
}

/// Get the web view for a tag.
async fn get_tag_web(
    State(state): StateType,
    headers: HeaderMap,
    uri: axum::http::Uri,
) -> impl axum::response::IntoResponse {
    tracing::debug!("{}", uri.path());
    let tag = &uri.path()["/tag/".len()..];
    let updater = state.updater.clone();
    let mut html = state.html.lock().await;
    return (
        HeaderMap::html_headers(),
        html.get(
            uri.path(),
            async move {
                updater.collect_tag(tag, headers.if_modified_since()).await
            },
            state.updater.clone(),
            state.config.clone(),
        )
        .await,
    );
}

/// Get the atom feed for a tag.
async fn get_tag_atom(
    State(state): StateType,
    headers: HeaderMap,
    uri: axum::http::Uri,
    axum::extract::Path(tag): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    tracing::debug!("{}", uri.path());
    let config = state.config.clone();
    let updater = state.updater.clone();
    let mut cache = state.cache.lock().await;
    let cache_behavior = headers.cache_behavior();
    return (
        HeaderMap::atom_headers(),
        cache
            .get(
                uri.path(),
                async move {
                    updater
                        .syndicate_tag(
                            &tag,
                            config,
                            headers.if_modified_since(),
                        )
                        .await
                },
                cache_behavior,
            )
            .await,
    );
}

/// Get the server config toml.
/// This is for convenience for anyone who may want to copy the feeds of a public
/// server.
/// This serves the parsed and re-exported config.toml. Any comments or extraneous
/// metadata is stripped.
async fn get_config(
    State(state): StateType,
) -> impl axum::response::IntoResponse {
    tracing::debug!("/config");
    let serialized: String = match toml::to_string_pretty(&state.config) {
        Ok(config) => config,
        Err(e) => {
            tracing::error!("Failed to serialize config: {e}");
            String::new()
        }
    };
    return (HeaderMap::toml_headers(), serialized);
}

/// Get the styles for the web view.
async fn get_styles(
    State(state): StateType,
) -> impl axum::response::IntoResponse {
    tracing::debug!("/styles.css");
    let html = state.html.lock().await;
    return (HeaderMap::css_headers(), (*html.styles).clone());
}

/// Get the robots.txt.
async fn get_robots_txt(
    State(state): StateType,
) -> impl axum::response::IntoResponse {
    tracing::debug!("/robots.txt");
    let html = state.html.lock().await;
    return (HeaderMap::plaintext_headers(), (*html.robots_txt).clone());
}

/// Get the slipstream favicon.
async fn get_favicon(
    State(state): StateType,
) -> impl axum::response::IntoResponse {
    tracing::debug!("/favicon.ico");
    let html = state.html.lock().await;
    return (HeaderMap::favicon_headers(), (*html.favicon).clone());
}
