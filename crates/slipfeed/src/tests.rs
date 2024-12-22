use super::*;

#[tokio::test]
async fn no_panics() {
    let mut updater = FeedUpdater::new(Duration::seconds(10));

    let mut rss_feeds = AggregateFeed::new();
    let hn = Feed::from_raw("https://news.ycombinator.com/rss");
    let hn_id = updater.add_feed(hn);
    rss_feeds.add_feed(hn_id);
    updater.add_feed(Feed::from_aggregate(rss_feeds.feeds));

    let mut atom_feeds = AggregateFeed::new();
    let newsboat = Feed::from_raw(
        // url: "https://xkcd.com/atom.xml".into(),
        "https://newsboat.org/news.atom",
    );
    let newsboat_id = updater.add_feed(newsboat);
    atom_feeds.add_feed(newsboat_id);
    updater.add_feed(Feed::from_aggregate(atom_feeds.feeds));

    updater.update().await;
    updater.iter().for_each(|entry| {
        println!("Entry: {:?}", entry);
    });

    // Feed::from(atom_feed).get_tags().for_each(|tag| {
    //     println!("Tag: {:?}", tag);
    // });
}
