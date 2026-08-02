#![allow(unused)]

use slipstream_feeds::prelude::Tag;

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
                slipfeed::EntryBuilder::new()
                    .title("Coding 0")
                    .author("Code Councel")
                    .build(),
                slipfeed::EntryBuilder::new()
                    .title("Rust 1")
                    .author("Joe Rustacean")
                    .build(),
                slipfeed::EntryBuilder::new().title("Zig 2").build(),
            ],
        });

        for (i, entry) in entries.entries.iter_mut().enumerate() {
            if i == 0 {
                entry.add_tag(&"dev".into());
            }
            if i == 1 {
                entry.add_tag(&"rust".into());
            }
            if i == 2 {
                entry.add_tag(&"zig".into());
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
    ) {
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
    }
}

async fn run(config: Config, tag: impl AsRef<str>) -> DatabaseEntryList {
    let mut task_manager = config
        .build_task_manager()
        .await
        .expect("Could not create updater.");
    {
        let mut updater = task_manager.updater.write().await;
        updater.add_feed(FakeFeed::test_set_a(), {
            let mut attr = slipfeed::FeedAttributes::new();
            attr.display_name = Arc::new("transformer".into());
            attr.filters
                .extend_from_slice(&config.global.filters.get_filters());
            attr
        });
    }

    let updater_handle = task_manager.handle().expect("Unable to get handle");
    let cancel_token = CancellationToken::new();
    let mut tasks = JoinSet::new();
    tasks.spawn(manage_tasks(
        task_manager,
        Arc::new(config),
        cancel_token.clone(),
    ));

    tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.2)).await;

    if tag.as_ref().is_empty() {
        updater_handle.collect_all(None).await
    } else {
        updater_handle.collect_tag(tag.as_ref(), None).await
    }
}

#[tokio::test]
async fn test_derivations() {
    tracing_subscriber::fmt::try_init().ok();
    let total = FakeFeed::test_set_a().entries.len();

    // Base case:
    let mut config = Config::default();
    let results = run(config, "dev").await;
    assert_eq!(results.iter_entries().count(), 1);

    // Derivation transformations:
    let mut derivations = BTreeMap::new();
    derivations.insert(Tag::new("dev"), {
        let mut set = HashSet::new();
        set.insert(Tag::new("rust"));
        set.insert(Tag::new("zig"));
        set
    });
    let mut config = Config::default();
    config.global.transforms.tag_derivations = Some(derivations);
    let results = run(config, "dev").await;
    assert_eq!(results.iter_entries().count(), 3);

    // Alias transformations:
    let mut aliases = BTreeMap::new();
    aliases.insert(Tag::new("dev"), {
        let mut set = HashSet::new();
        set.insert(Tag::new("rust"));
        set.insert(Tag::new("zig"));
        set
    });
    let mut config = Config::default();
    config.global.transforms.tag_aliases = Some(aliases.clone());
    let results = run(config, "dev").await;
    assert_eq!(results.iter_entries().count(), 3);
    let mut config = Config::default();
    config.global.transforms.tag_aliases = Some(aliases.clone());
    let results = run(config, "rust").await;
    assert_eq!(results.iter_entries().count(), 0);
    let mut config = Config::default();
    config.global.transforms.tag_aliases = Some(aliases.clone());
    let results = run(config, "zig").await;
    assert_eq!(results.iter_entries().count(), 0);

    // Author transformations:
    let mut config = Config::default();
    config.global.transforms.author = Some("{feed}".into());
    let results = run(config, "").await;
    assert_eq!(
        results
            .iter_entries()
            .filter(|e| e.author().contains("transformer"))
            .count(),
        3
    );
}
