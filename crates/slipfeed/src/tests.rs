use super::*;

#[tokio::test]
async fn no_panics() {
    // let mut feed = AggregateFeed::new();
    let mut updater = FeedUpdater::new(Duration::seconds(10));

    // let mut rss_feed = AggregateFeed::new();
    // let hacker_news = updater.add_feed(RawFeed {
    //     url: "https://news.ycombinator.com/rss".into(),
    //     // tags: vec!["Hacking".into(), "RSS".into()],
    // });
    // rss_feed.add_feed(hacker_news);
    // updater.add_feed(rss_feed);

    // let mut atom_feed = AggregateFeed::new();
    // let newsboat = updater.add_feed(RawFeed {
    //     // url: "https://xkcd.com/atom.xml".into(),
    //     url: "https://newsboat.org/news.atom".into(),
    //     // tags: vec!["Reader".into(), "ATOM".into()],
    // });
    // atom_feed.add_feed(newsboat);
    // updater.add_feed(atom_feed);

    // updater.update().await;
    // updater.entries.iter().for_each(|entry| {
    //     println!("Entry: {:?}", entry);
    // });

    // Feed::from(atom_feed).get_tags().for_each(|tag| {
    //     println!("Tag: {:?}", tag);
    // });
}
