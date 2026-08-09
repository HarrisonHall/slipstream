#![allow(unused)]

use super::*;

#[derive(Clone, Debug)]
struct FakeFeed {
    entries: Vec<slipfeed::Entry>,
}

impl FakeFeed {
    fn new(entries: Vec<slipfeed::Entry>) -> Box<Self> {
        Box::new(Self { entries })
    }

    fn test_set_a() -> Box<Self> {
        let mut entries = Box::new(Self {
            entries: vec![
                slipfeed::EntryBuilder::new().title("Hello World 0").build(),
                slipfeed::EntryBuilder::new().title("Cool Stuff 1").build(),
                slipfeed::EntryBuilder::new().title("Hot Stuff 2").build(),
                slipfeed::EntryBuilder::new()
                    .title("Business: A New Venture 3")
                    .content("A worthy business venture for all money people.")
                    .build(),
                slipfeed::EntryBuilder::new()
                    .title("4")
                    .source("https://business4.net/turtle")
                    .content("Business if for cash turtles. Watch here: https://youtube.com/turtle-videos")
                    .build(),
                slipfeed::EntryBuilder::new()
                    .title("5")
                    .source("https://pizza.business5.dev/hotdog")
                    .build(),
                slipfeed::EntryBuilder::new()
                    .title("6: Coder Society")
                    .source("https://coders.society.dev")
                    .content("Rust week")
                    .build(),
                slipfeed::EntryBuilder::new()
                    .title("7: Cool Cool Mountain")
                    .content("Rustic views")
                    .build(),
                slipfeed::EntryBuilder::new()
                    .title("8: Adventure Island")
                    .content("A fun new attraction at Theme Park 8!")
                    .build(),
            ],
        });

        for entry in &mut entries.entries {
            if entry.title().to_lowercase().contains("coder") {
                entry.add_tag(&"dev".into());
            }
            if entry.content().to_lowercase().contains("rust ") {
                entry.add_tag(&"Rust".into());
            }
            if entry.content().to_lowercase().contains("rustic") {
                entry.add_tag(&"rustic".into());
            }
        }

        entries
    }
}

#[slipfeed::feed_trait]
impl slipfeed::Feed for FakeFeed {
    async fn update(
        &mut self,
        ctx: &slipfeed::UpdaterContext,
        attr: &slipfeed::FeedAttributes,
    ) -> Result<(), slipfeed::UpdateError> {
        for entry in &self.entries {
            match ctx.sender.send((
                entry.clone(),
                slipfeed::FeedRef {
                    id: ctx.feed_id,
                    name: attr.display_name.clone(),
                },
            )) {
                Ok(()) => {}
                Err(e) => tracing::error!("Failed to send entry: {e}"),
            }
        }

        Ok(())
    }
}

async fn run(config: Config) -> DatabaseEntryList {
    let mut updater = config
        .build_task_manager()
        .await
        .expect("Could not create updater.");
    {
        let mut updater = updater.updater.write().await;
        updater.add_feed(FakeFeed::test_set_a(), {
            let mut attr = slipfeed::FeedAttributes::new();
            attr.filters
                .extend_from_slice(&config.global.filters.get_filters());
            attr
        });
    }

    let updater_handle = updater.handle().expect("Unable to get handle");
    let cancel_token = CancellationToken::new();
    let mut tasks = JoinSet::new();
    tasks.spawn(manage_tasks(updater, cancel_token.clone()));

    tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.1)).await;

    let all = updater_handle.collect_all(None).await;

    all
}

#[tokio::test]
async fn test_filters() {
    tracing_subscriber::fmt::try_init().ok();
    let total = FakeFeed::test_set_a().entries.len();

    // Generic filters:
    let mut config = Config::default();
    config.global.filters.exclude = Some(vec!["hello".into(), "hot".into()]);
    let results = run(config).await;
    assert_eq!(results.iter_entries().count(), total - 3);
    let mut config = Config::default();
    config.global.filters.include = Some(vec!["hello".into(), "hot".into()]);
    let results = run(config).await;
    assert_eq!(results.iter_entries().count(), 3);
    let mut config = Config::default();
    config.global.filters.exclude = Some(vec!["hello".into(), "hot".into()]);
    config.global.filters.include = Some(vec!["hello".into(), "hot".into()]);
    let results = run(config).await;
    assert_eq!(results.iter_entries().count(), 0);
    let mut config = Config::default();
    config.global.filters.include_all =
        Some(vec!["stuff".into(), "hot".into()]);
    let results = run(config).await;
    assert_eq!(results.iter_entries().count(), 1);

    // Title filters:
    let mut config = Config::default();
    config.global.filters.exclude_titles = Some(vec!["hello".into()]);
    let results = run(config).await;
    assert_eq!(results.iter_entries().count(), total - 1);
    let mut config = Config::default();
    config.global.filters.include_titles =
        Some(vec!["cool".into(), "hot".into()]);
    let results = run(config).await;
    assert_eq!(results.iter_entries().count(), 3);

    // Regex filters:
    let mut config = Config::default();
    config.global.filters.exclude_re = Some(vec!["[vV]enture".into()]);
    let results = run(config).await;
    assert_eq!(results.iter_entries().count(), total - 2);
    let mut config = Config::default();
    config.global.filters.include_re =
        Some(vec![r"\.dev".into(), r"[58]".into()]);
    let results = run(config).await;
    assert_eq!(results.iter_entries().count(), 3);
    let mut config = Config::default();
    config.global.filters.include_re = Some(vec![r"[0-9]*".into()]);
    let results = run(config).await;
    assert_eq!(results.iter_entries().count(), total);

    // Content filters:
    let mut config = Config::default();
    config.global.filters.exclude_contents = Some(vec!["money".into()]);
    let results = run(config).await;
    assert_eq!(results.iter_entries().count(), total - 1);

    // Tag filters:
    let mut config = Config::default();
    config.global.filters.exclude_tags = Some(vec!["rust".into()]);
    let results = run(config).await;
    assert_eq!(results.iter_entries().count(), total - 2);
    let mut config = Config::default();
    config.global.filters.exclude_tags_strict = Some(vec!["rust".into()]);
    let results = run(config).await;
    assert_eq!(results.iter_entries().count(), total - 1);
    let mut config = Config::default();
    config.global.filters.include_tags = Some(vec!["rust".into()]);
    let results = run(config).await;
    assert_eq!(results.iter_entries().count(), 2);
    let mut config = Config::default();
    config.global.filters.include_tags_strict = Some(vec!["rust".into()]);
    let results = run(config).await;
    assert_eq!(results.iter_entries().count(), 1);
}
