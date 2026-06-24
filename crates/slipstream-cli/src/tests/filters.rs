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
        Box::new(Self {
            entries: vec![
                slipfeed::EntryBuilder::new().title("Hello World 0").build(),
                slipfeed::EntryBuilder::new().title("Cool Stuff 1").build(),
                slipfeed::EntryBuilder::new().title("Hot Stuff 2").build(),
                slipfeed::EntryBuilder::new()
                    .title("BusinessA new venture 3")
                    .build(),
                slipfeed::EntryBuilder::new()
                    .title("4")
                    .source("https://business4.net/turtle")
                    .build(),
                slipfeed::EntryBuilder::new()
                    .title("5")
                    .source("https://pizza.business5.dev/hotdog")
                    .build(),
            ],
        })
    }
}

#[slipfeed::feed_trait]
impl slipfeed::Feed for FakeFeed {
    async fn update(
        &mut self,
        ctx: &slipfeed::UpdaterContext,
        attr: &slipfeed::FeedAttributes,
    ) {
        for entry in &self.entries {
            let passes_filters = attr.passes_filters(self, entry);
            if !passes_filters {
                continue;
            }

            ctx.sender
                .send((
                    entry.clone(),
                    slipfeed::FeedRef {
                        id: ctx.feed_id,
                        name: attr.display_name.clone(),
                    },
                ))
                .ok();
        }
    }
}

#[tokio::test]
async fn test_filters() {
    tracing_subscriber::fmt::try_init().ok();

    let mut config = Config::default();
    config.global.filters.exclude_title_words = Some(vec!["hello".into()]);
    // config.global.filters.exclude_substrings =
    //     Some(vec!["hello".into(), "hot".into()]);

    let mut updater =
        config.updater().await.expect("Could not create updater.");
    {
        let mut updater = updater.updater.write().await;
        updater.add_feed(
            FakeFeed::test_set_a(),
            {
                let mut attr = slipfeed::FeedAttributes::new();
                attr.filters
                    .extend_from_slice(&config.global.filters.get_filters());
                attr
            },
            // slipfeed::FeedAttributes {
            //     display_name: Arc::new("Test".into()),
            //     timeout: slipfeed::Duration::from_hours(10),
            //     freq: None,
            //     step: 1,
            //     tags: std::collections::HashSet::from([slipfeed::Tag::new(
            //         "test",
            //     )]),
            //     filters: vec![],
            //     keep_empty: false,
            //     apply_tags: true,
            //     headers: BTreeMap::new(),
            // },
        );

        // let x = updater.update().await;
        // assert!(x.len() == 5);
    }

    let updater_handle = updater.handle().expect("Unable to get handle");
    let cancel_token = CancellationToken::new();
    let mut tasks = JoinSet::new();
    tasks.spawn(update(updater, Arc::new(config), cancel_token.clone()));

    tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.5)).await;
    let all = updater_handle.collect_all(None).await;
    assert_eq!(all.iter_entries().count(), 5);
}
