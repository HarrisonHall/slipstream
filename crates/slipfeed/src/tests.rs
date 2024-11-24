use super::*;

use std::iter::{Chain, Sum};

use chrono::Duration;

#[tokio::test]
async fn no_panics() {
    let mut feed = AggregateFeed::new();

    let mut rss_feed = AggregateFeed::new();
    rss_feed.add_feed(RawFeed {
        name: "Foo".into(),
        url: "https://news.ycombinator.com/rss".into(),
        tags: vec!["Hacking".into(), "RSS".into()],
    });
    rss_feed
        .updater()
        .frequency(Duration::seconds(1))
        .call()
        .update()
        .await;

    let mut atom_feed = AggregateFeed::new();
    atom_feed.add_feed(RawFeed {
        name: "Bar".into(),
        // url: "https://xkcd.com/atom.xml".into(),
        url: "https://newsboat.org/news.atom".into(),
        tags: vec!["Reader".into(), "ATOM".into()],
    });
    atom_feed
        .updater()
        .frequency(Duration::seconds(1))
        .call()
        .update()
        .await;

    feed.add_feed(rss_feed.clone());
    feed.add_feed(atom_feed.clone());
    let mut updater = feed.updater().frequency(Duration::seconds(1)).call();
    updater.update().await;
    updater.entries.iter().for_each(|entry| {
        println!("Entry: {:?}", entry);
    });

    Feed::from(atom_feed).get_tags().for_each(|tag| {
        println!("Tag: {:?}", tag);
    });
}
