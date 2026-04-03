use super::*;

#[tokio::test]
async fn standard_syndications() {
    tracing_subscriber::fmt::try_init().ok();

    let mut rss_updater = Updater::new(Duration::from_seconds(1_000), 5);
    let hn = StandardSyndication::new("https://news.ycombinator.com/rss");
    let hn_id = rss_updater.add_feed(
        hn,
        FeedAttributes {
            display_name: Arc::new("HackerNews".into()),
            timeout: Duration::from_hours(10),
            freq: None,
            step: 1,
            tags: std::collections::HashSet::from([Tag::new("rss")]),
            filters: vec![],
            keep_empty: false,
            apply_tags: true,
            headers: BTreeMap::new(),
        },
    );
    assert!(hn_id.0 == 1);

    let mut atom_updater = Updater::new(Duration::from_seconds(1_000), 5);
    let newsboat = StandardSyndication::new("https://newsboat.org/news.atom");
    let newsboat_id = atom_updater.add_feed(
        newsboat,
        FeedAttributes {
            display_name: Arc::new("NewsBoat".into()),
            timeout: Duration::from_days(365),
            freq: None,
            step: 1,
            tags: std::collections::HashSet::from([Tag::new("atom")]),
            filters: vec![],
            keep_empty: false,
            apply_tags: true,
            headers: BTreeMap::new(),
        },
    );
    assert!(newsboat_id.0 == 1);

    let rss_entries = rss_updater.update().await;
    let atom_entries = atom_updater.update().await;

    assert!(rss_entries.len() > 0);
    assert!(atom_entries.len() > 0);

    for entry in rss_entries.as_slice() {
        assert!(entry.has_tag(&Tag::new("rss")));
    }
    for entry in atom_entries.as_slice() {
        assert!(entry.has_tag(&Tag::new("atom")));
    }
}

#[tokio::test]
async fn mastodon() {
    tracing_subscriber::fmt::try_init().ok();

    let mut updater = Updater::new(Duration::from_seconds(1_000), 5);
    let mast = MastodonFeed::new(
        "https://mastodon.social",
        MastodonFeedType::PublicTimeline,
        None,
    );
    updater.add_feed(
        mast,
        FeedAttributes {
            display_name: Arc::new("Mastodon".into()),
            timeout: Duration::from_days(365),
            freq: None,
            step: 1,
            tags: std::collections::HashSet::from([Tag::new("mastodon")]),
            filters: vec![],
            keep_empty: false,
            apply_tags: true,
            headers: BTreeMap::new(),
        },
    );

    let entries = updater.update().await;
    assert!(entries.len() > 0);
}
