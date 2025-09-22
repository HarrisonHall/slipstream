use super::*;

#[tokio::test]
async fn config_serialization() {
    tracing_subscriber::fmt::try_init().ok();

    // Individual types.
    let sc = ServeConfig::default();
    let sc_toml = toml::to_string_pretty(&sc).unwrap();
    let _sc: ServeConfig = toml::from_str(&sc_toml).unwrap();

    let rc = ReadConfig::default();
    let rc_toml = toml::to_string_pretty(&rc).unwrap();
    let _rc: ReadConfig = toml::from_str(&rc_toml).unwrap();

    let fd = FeedDefinition::from_feed(RawFeed::Raw {
        url: "https://example.com".into(),
    });
    let fd_toml = toml::to_string_pretty(&fd).unwrap();
    let _fd: ReadConfig = toml::from_str(&fd_toml).unwrap();

    let fd = FeedDefinition::from_feed(RawFeed::Aggregate {
        feeds: vec!["foo".into()],
    });
    let fd_toml = toml::to_string_pretty(&fd).unwrap();
    let _fd: ReadConfig = toml::from_str(&fd_toml).unwrap();

    let fd = FeedDefinition::from_feed(RawFeed::MastodonStatuses {
        mastodon: "https://mastodon.social".into(),
        feed_type: MastodonFeedType::HomeTimeline,
        token: None,
    });
    let fd_toml = toml::to_string_pretty(&fd).unwrap();
    let _fd: ReadConfig = toml::from_str(&fd_toml).unwrap();

    let fd = FeedDefinition::from_feed(RawFeed::MastodonUserStatuses {
        mastodon: "https://mastodon.social".into(),
        user: "Foo".into(),
        token: None,
    });
    let fd_toml = toml::to_string_pretty(&fd).unwrap();
    let _fd: ReadConfig = toml::from_str(&fd_toml).unwrap();

    let config_path = "../../examples/config/slipreader.toml";
    let config_data = std::fs::read_to_string(&config_path).unwrap();
    let config = toml::from_str::<Config>(&config_data).unwrap();
    let e: Result<String, toml::ser::Error> = toml::to_string_pretty(&config);
    match &e {
        Ok(_) => {}
        Err(e) => eprintln!("Error! {:?}", e),
    };
    assert!(e.is_ok());

    let config_path = "../../examples/config/slipstream.toml";
    let config_data = std::fs::read_to_string(&config_path).unwrap();
    let config = toml::from_str::<Config>(&config_data).unwrap();
    let e: Result<String, toml::ser::Error> = toml::to_string_pretty(&config);
    match &e {
        Ok(_) => {}
        Err(e) => eprintln!("Error! {:?}", e),
    };
    assert!(e.is_ok());
}
