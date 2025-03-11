//! Serve mode.

use super::*;

use axum::extract::State;

mod web;

use web::*;

/// Serve slipstream over http.
pub async fn serve(
    port: Option<u16>,
    config: Arc<Config>,
    updater: Arc<Mutex<Updater>>,
) -> Result<()> {
    // Create caches.
    let duration = slipfeed::Duration::from_seconds(match config.cache {
        Some(freq) => freq.as_secs(),
        None => 120,
    });
    let cache = Arc::new(Mutex::new(Cache::new(duration.clone())));
    let html = Arc::new(Mutex::new(HtmlServer::new(duration)?));

    // Create server.
    let app = axum::Router::new()
        .route("/", axum::routing::get(get_all_html))
        .route("/config", axum::routing::get(get_config))
        .route("/all", axum::routing::get(get_all_html))
        .route("/all/feed", axum::routing::get(get_all_feed))
        .route("/feed/:feed", axum::routing::get(get_feed_html))
        .route("/feed/:feed/feed", axum::routing::get(get_feed_feed))
        .route("/tag/:tag", axum::routing::get(get_tag_html))
        .route("/tag/:tag/feed", axum::routing::get(get_tag_feed))
        .route("/styles.css", axum::routing::get(get_styles))
        .route("/robots.txt", axum::routing::get(get_robots_txt))
        .route("/favicon.ico", axum::routing::get(get_favicon))
        .with_state(Arc::new(SFState {
            updater,
            config: config.clone(),
            cache,
            html,
        }));
    let port = port.unwrap_or(config.port.unwrap_or(DEFAULT_PORT));
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect(&format!("Unable to bind to port {}", port));

    // Serve.
    tracing::info!("slipstream serve");
    tracing::info!("Serving feeds @ 0.0.0.0:{}", port);
    axum::serve(listener, app).await.expect("Error serving.");
    Ok(())
}

#[derive(Clone)]
struct SFState {
    updater: Arc<Mutex<Updater>>,
    config: Arc<Config>,
    cache: Arc<Mutex<Cache>>,
    html: Arc<Mutex<HtmlServer>>,
}
type StateType = axum::extract::State<Arc<SFState>>;

async fn get_all_html(
    State(state): StateType,
) -> impl axum::response::IntoResponse {
    tracing::debug!("/all");
    let config = &state.config;
    let updater = state.updater.lock().await;
    let mut html = state.html.lock().await;
    return (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html.get("/all", async move { updater.collect_all(config) })
            .await,
    );
}

async fn get_all_feed(
    State(state): StateType,
) -> impl axum::response::IntoResponse {
    tracing::debug!("/all/feed");
    let config = &state.config;
    let updater = state.updater.lock().await;
    let mut cache = state.cache.lock().await;
    return (
        [(axum::http::header::CONTENT_TYPE, "application/atom+xml")],
        cache
            .get("/all", async move { updater.syndicate_all(config) })
            .await,
    );
}

async fn get_feed_html(
    State(state): StateType,
    uri: axum::http::Uri,
) -> impl axum::response::IntoResponse {
    tracing::debug!("{}", uri.path());
    let feed = &uri.path()["/feed/".len()..];
    let config = &state.config;
    let updater = state.updater.lock().await;
    let mut html = state.html.lock().await;
    return (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html.get(
            uri.path(),
            async move { updater.collect_feed(feed, config) },
        )
        .await,
    );
}

async fn get_feed_feed(
    State(state): StateType,
    uri: axum::http::Uri,
    axum::extract::Path(feed): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    tracing::debug!("{}", uri.path());
    let config = &state.config;
    let updater = state.updater.lock().await;
    let mut cache = state.cache.lock().await;
    return (
        [(axum::http::header::CONTENT_TYPE, "application/atom+xml")],
        cache
            .get(
                uri.path(),
                async move { updater.syndicate_feed(&feed, config) },
            )
            .await,
    );
}

async fn get_tag_html(
    State(state): StateType,
    uri: axum::http::Uri,
) -> impl axum::response::IntoResponse {
    tracing::debug!("{}", uri.path());
    let tag = &uri.path()["/tag/".len()..];
    let config = &state.config;
    let updater = state.updater.lock().await;
    let mut html = state.html.lock().await;
    return (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html.get(uri.path(), async move { updater.collect_tag(tag, config) })
            .await,
    );
}

async fn get_tag_feed(
    State(state): StateType,
    uri: axum::http::Uri,
    axum::extract::Path(tag): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    tracing::debug!("{}", uri.path());
    let config = &state.config;
    let updater = state.updater.lock().await;
    let mut cache = state.cache.lock().await;
    return (
        [(axum::http::header::CONTENT_TYPE, "application/atom+xml")],
        cache
            .get(
                uri.path(),
                async move { updater.syndicate_tag(&tag, config) },
            )
            .await,
    );
}

async fn get_config(
    State(state): StateType,
) -> impl axum::response::IntoResponse {
    tracing::debug!("/config");
    let mut with_feed = String::new();
    state
        .config
        .serialize(toml::Serializer::new(&mut with_feed))
        .ok();
    return (
        [(axum::http::header::CONTENT_TYPE, "application/toml")],
        with_feed,
    );
}

async fn get_styles(
    State(state): StateType,
) -> impl axum::response::IntoResponse {
    tracing::debug!("/styles.css");
    let html = state.html.lock().await;
    return (
        [(axum::http::header::CONTENT_TYPE, "text/css")],
        (*html.styles).clone(),
    );
}

async fn get_robots_txt(
    State(state): StateType,
) -> impl axum::response::IntoResponse {
    tracing::debug!("/robots.txt");
    let html = state.html.lock().await;
    return (
        [(axum::http::header::CONTENT_TYPE, "text/plain")],
        (*html.robots_txt).clone(),
    );
}

async fn get_favicon(
    State(state): StateType,
) -> impl axum::response::IntoResponse {
    tracing::debug!("/robots.txt");
    let html = state.html.lock().await;
    return (
        [(axum::http::header::CONTENT_TYPE, "image/x-icon")],
        (*html.favicon).clone(),
    );
}
