//! Slipstream updater.

use super::*;

use tokio::sync::oneshot;

/// Run the slipstream updater.
pub async fn update(
    mut updater: Updater,
    config: Arc<Config>,
    cancel_token: CancellationToken,
) -> Result<()> {
    // We don't want to cancel the updater's updater update future working on other
    // jobs. We convert this loop into a task and only cancel on quit.
    let updater_task: tokio::task::JoinHandle<()> =
        tokio::task::spawn(run_updater(
            updater.updater.clone(),
            updater.entry_db.clone(),
            cancel_token.clone(),
        ));

    // Continue updating and responding to requests until cancelled.
    'update: loop {
        tokio::select! {
            command = updater.to_updater_receiver.recv() => {
                if let Some(command) = command {
                    updater.handle_command(command, &config).await;
                }
            },
            _ = cancel_token.cancelled() => break 'update,
        }
    }

    updater_task.abort();

    Ok(())
}

async fn run_updater(
    internal_updater: Arc<RwLock<slipfeed::Updater>>,
    entry_db: Option<Arc<Database>>,
    cancel_token: CancellationToken,
) {
    while !cancel_token.is_cancelled() {
        let entries = {
            let mut slipfeed_updater = internal_updater.write().await;
            slipfeed_updater.update().await
        };
        for entry in entries.as_slice() {
            if let Some(entry_db) = &entry_db {
                entry_db.insert_slipfeed_entry(entry).await;
            }
        }
    }
    ()
}

/// Slipstream updater.
pub struct Updater {
    /// Underlying slipfeed updater.
    pub updater: Arc<RwLock<slipfeed::Updater>>,
    /// Map feeds by name to slipfeed id.
    pub feeds: HashMap<String, slipfeed::FeedId>,
    /// Map slipfeed ids to the feed names.
    pub feeds_ids: HashMap<slipfeed::FeedId, String>,
    /// Global filters (applies to everything).
    pub global_filters: Vec<slipfeed::Filter>,
    /// All filters (applies to the /all feed).
    pub all_filters: Vec<slipfeed::Filter>,
    /// The entry database.
    /// This allows persistance between slipstream sessions.
    pub entry_db: Option<Arc<Database>>,
    /// Handle's sender.
    to_updater_sender: Sender<UpdaterRequest>,
    /// Updater's receiver.
    to_updater_receiver: Receiver<UpdaterRequest>,
}

impl Updater {
    /// Get handle to updater.
    pub fn handle(&mut self) -> Result<UpdaterHandle> {
        Ok(UpdaterHandle {
            to_updater_sender: self.to_updater_sender.clone(),
        })
    }

    /// Handle command.
    async fn handle_command(
        &self,
        command: UpdaterRequest,
        config: &Arc<Config>,
    ) {
        match command {
            UpdaterRequest::EntryUpdate { entry_id, tags } => {
                if let Some(entry_db) = &self.entry_db {
                    if let Some(tags) = tags {
                        entry_db.update_tags(entry_id, tags).await;
                    }
                }
            }
            UpdaterRequest::CommandUpdate {
                entry_id,
                command,
                result,
                output,
            } => {
                if let Some(entry_db) = &self.entry_db {
                    entry_db
                        .store_command_result(
                            entry_id,
                            command,
                            output,
                            result == 0,
                        )
                        .await;
                }
            }
            UpdaterRequest::EntriesSearch {
                tx,
                criteria,
                offset,
            } => {
                if let Some(entry_db) = &self.entry_db {
                    // TODO: custom search count.
                    tx.send(entry_db.get_entries(criteria, 128, offset).await)
                        .ok();
                };
            }
            UpdaterRequest::FeedFetch { tx, options } => {
                if let Some(entry_db) = &self.entry_db {
                    let entries = match options {
                        FeedFetchOptions::All {
                            since,
                            modified_since,
                        } => {
                            let unfiltered_entries = entry_db
                                .get_entries(
                                    vec![DatabaseSearch::Latest],
                                    config.global.limits.max(),
                                    match (since, modified_since) {
                                        (Some(since), None) => {
                                            OffsetCursor::from(Some(since))
                                        }
                                        (None, Some(modified_since)) => {
                                            OffsetCursor::ModifiedAfter(
                                                modified_since,
                                            )
                                        }
                                        (Some(_), Some(_)) => {
                                            OffsetCursor::LatestTimestamp
                                        }
                                        (None, None) => {
                                            OffsetCursor::LatestTimestamp
                                        }
                                    },
                                )
                                .await;
                            let mut entries = DatabaseEntryList::new(
                                config.global.limits.max(),
                            );
                            for entry in unfiltered_entries.iter() {
                                if config.global.limits.too_old(entry.date()) {
                                    continue;
                                }
                                if !self.passes_global_filters(&entry) {
                                    continue;
                                }
                                if !self.passes_all_filters(&entry) {
                                    continue;
                                }
                                entries.add(entry.clone()).ok();
                            }
                            entries
                        }
                        FeedFetchOptions::Tag { tag, since } => {
                            let unfiltered_entries = entry_db
                                .get_entries(
                                    vec![DatabaseSearch::Tag(tag)],
                                    config.global.limits.max(),
                                    OffsetCursor::from(since),
                                )
                                .await;
                            let mut entries = DatabaseEntryList::new(
                                config.global.limits.max(),
                            );
                            for entry in unfiltered_entries.iter() {
                                if config.global.limits.too_old(entry.date()) {
                                    continue;
                                }
                                if !self.passes_global_filters(&entry) {
                                    continue;
                                }
                                entries.add(entry.clone()).ok();
                            }
                            entries
                        }
                        FeedFetchOptions::Feed { feed, since } => {
                            if let (Some(_feed_id), Some(feed_def)) =
                                (self.feeds.get(&feed), config.feed(&feed))
                            {
                                let unfiltered_entries = entry_db
                                    .get_entries(
                                        vec![DatabaseSearch::Feed(
                                            feed.clone(),
                                        )],
                                        config.global.limits.max(),
                                        OffsetCursor::from(since),
                                    )
                                    .await;
                                let mut entries = DatabaseEntryList::new(
                                    feed_def.options().max(),
                                );
                                for entry in unfiltered_entries.iter() {
                                    if config
                                        .global
                                        .limits
                                        .too_old(entry.date())
                                    {
                                        continue;
                                    }
                                    if feed_def.options().too_old(entry.date())
                                    {
                                        continue;
                                    }
                                    if !self.passes_global_filters(&entry) {
                                        continue;
                                    }
                                    // NOTE: Individual feed filters are already checked by the underlying
                                    // slipfeed updater.
                                    entries.add(entry.clone()).ok();
                                }
                                entries
                            } else {
                                DatabaseEntryList::new(0)
                            }
                        }
                    };
                    tx.send(entries).ok();
                };
            }
            UpdaterRequest::FeedName { tx, feed } => {
                // config.feed(feed)
                tx.send(self.feeds_ids.get(&feed).map(|f| f.clone())).ok();
            }
        }
    }

    /// Check if entry passes the global filters.
    pub fn passes_global_filters(&self, entry: &slipfeed::Entry) -> bool {
        let feed = NoopFeed;
        self.global_filters.iter().all(|f| f(&feed, entry))
    }

    /// Check if entry passes the all filters.
    pub fn passes_all_filters(&self, entry: &slipfeed::Entry) -> bool {
        let feed = NoopFeed;
        self.all_filters.iter().all(|f| f(&feed, entry))
    }
}

impl Default for Updater {
    fn default() -> Self {
        let (to_updater_sender, to_updater_receiver) = channel(10);
        Self {
            updater: Arc::new(RwLock::new(slipfeed::Updater::default())),
            feeds: HashMap::default(),
            feeds_ids: HashMap::default(),
            global_filters: Vec::default(),
            all_filters: Vec::default(),
            entry_db: None,
            to_updater_sender,
            to_updater_receiver,
        }
    }
}

/// Message used to communicate with the database handler.
#[derive(Debug)]
enum UpdaterRequest {
    EntryUpdate {
        entry_id: EntryDbId,
        tags: Option<Vec<slipfeed::Tag>>,
    },
    EntriesSearch {
        tx: oneshot::Sender<DatabaseEntryList>,
        criteria: Vec<DatabaseSearch>,
        offset: OffsetCursor,
    },
    FeedFetch {
        tx: oneshot::Sender<DatabaseEntryList>,
        options: FeedFetchOptions,
    },
    FeedName {
        tx: oneshot::Sender<Option<String>>,
        feed: slipfeed::FeedId,
    },
    CommandUpdate {
        entry_id: EntryDbId,
        command: String,
        result: i32,
        output: String,
    },
}

#[derive(Debug, Clone)]
enum FeedFetchOptions {
    All {
        since: Option<slipfeed::DateTime>,
        modified_since: Option<slipfeed::DateTime>,
    },
    Feed {
        feed: String,
        since: Option<slipfeed::DateTime>,
    },
    Tag {
        tag: String,
        since: Option<slipfeed::DateTime>,
    },
}

#[derive(Clone)]
pub struct UpdaterHandle {
    /// Handle's sender.
    to_updater_sender: Sender<UpdaterRequest>,
}

impl UpdaterHandle {
    async fn send(&self, message: UpdaterRequest) {
        let res = self.to_updater_sender.send(message).await;
        if let Err(e) = res {
            tracing::error!("Failed to send: {}", e);
        }
    }

    /// Search for entries from a feed.
    pub async fn search(
        &self,
        criteria: Vec<DatabaseSearch>,
        offset: OffsetCursor,
    ) -> DatabaseEntryList {
        let (tx, rx) = oneshot::channel::<DatabaseEntryList>();
        self.send(UpdaterRequest::EntriesSearch {
            tx,
            criteria,
            offset,
        })
        .await;
        match rx.await {
            Ok(data) => data,
            Err(e) => {
                tracing::error!("Failed to search entries: {}", e);
                DatabaseEntryList::new(0)
            }
        }
    }

    /// Collect the /all feed.
    pub async fn collect_all(
        &self,
        modified_since: Option<slipfeed::DateTime>,
    ) -> DatabaseEntryList {
        let (tx, rx) = oneshot::channel::<DatabaseEntryList>();
        self.send(UpdaterRequest::FeedFetch {
            tx,
            options: FeedFetchOptions::All {
                since: None,
                modified_since,
            },
        })
        .await;
        match rx.await {
            Ok(data) => data,
            Err(e) => {
                tracing::error!("Failed to collect_all: {}", e);
                DatabaseEntryList::new(0)
            }
        }
    }

    /// Convert the /all feed into an atom feed.
    pub async fn syndicate_all(
        &self,
        config: Arc<Config>,
        modified_since: Option<slipfeed::DateTime>,
    ) -> String {
        let (tx, rx) = oneshot::channel::<DatabaseEntryList>();
        self.send(UpdaterRequest::FeedFetch {
            tx,
            options: FeedFetchOptions::All {
                since: None,
                modified_since,
            },
        })
        .await;
        match rx.await {
            Ok(data) => data.syndicate("All", &config),
            Err(e) => {
                tracing::error!("Failed to syndicate_all: {}", e);
                String::new()
            }
        }
    }

    /// Collect the /feed feed.
    pub async fn collect_feed(
        &self,
        feed: impl Into<String>,
        since: Option<slipfeed::DateTime>,
    ) -> DatabaseEntryList {
        let (tx, rx) = oneshot::channel::<DatabaseEntryList>();
        self.send(UpdaterRequest::FeedFetch {
            tx,
            options: FeedFetchOptions::Feed {
                feed: feed.into(),
                since,
            },
        })
        .await;
        match rx.await {
            Ok(data) => data,
            Err(e) => {
                tracing::error!("Failed to collect_feed: {}", e);
                DatabaseEntryList::new(0)
            }
        }
    }

    /// Convert the /feed feed into an atom feed.
    pub async fn syndicate_feed(
        &self,
        feed: impl Into<String>,
        config: Arc<Config>,
        since: Option<slipfeed::DateTime>,
    ) -> String {
        let feed = feed.into();
        let (tx, rx) = oneshot::channel::<DatabaseEntryList>();
        self.send(UpdaterRequest::FeedFetch {
            tx,
            options: FeedFetchOptions::Feed {
                feed: feed.clone(),
                since,
            },
        })
        .await;
        match rx.await {
            Ok(data) => data.syndicate(&feed, &config),
            Err(e) => {
                tracing::error!("Failed to syndicate_tag: {}", e);
                String::new()
            }
        }
    }

    /// Collect the /tag feed.
    pub async fn collect_tag(
        &self,
        tag: impl Into<String>,
        since: Option<slipfeed::DateTime>,
    ) -> DatabaseEntryList {
        let (tx, rx) = oneshot::channel::<DatabaseEntryList>();
        self.send(UpdaterRequest::FeedFetch {
            tx,
            options: FeedFetchOptions::Tag {
                tag: tag.into(),
                since,
            },
        })
        .await;
        match rx.await {
            Ok(data) => data,
            Err(e) => {
                tracing::error!("Failed to collect_tag: {}", e);
                DatabaseEntryList::new(0)
            }
        }
    }

    /// Convert the /tag feed into an atom feed.
    pub async fn syndicate_tag(
        &self,
        tag: impl Into<String>,
        config: Arc<Config>,
        since: Option<slipfeed::DateTime>,
    ) -> String {
        let tag = tag.into();
        let (tx, rx) = oneshot::channel::<DatabaseEntryList>();
        self.send(UpdaterRequest::FeedFetch {
            tx,
            options: FeedFetchOptions::Tag {
                tag: tag.clone(),
                since,
            },
        })
        .await;
        match rx.await {
            Ok(data) => data.syndicate(&tag, &config),
            Err(e) => {
                tracing::error!("Failed to syndicate_tag: {}", e);
                String::new()
            }
        }
    }

    /// Get the feed name from id.
    #[allow(unused)]
    pub async fn feed_name(&self, id: slipfeed::FeedId) -> Option<String> {
        let (tx, rx) = oneshot::channel::<Option<String>>();
        self.send(UpdaterRequest::FeedName { tx, feed: id }).await;
        match rx.await {
            Ok(data) => data,
            Err(e) => {
                tracing::error!("Failed to feed_name: {}", e);
                None
            }
        }
    }

    pub async fn update_tags(
        &self,
        entry_id: EntryDbId,
        tags: Vec<slipfeed::Tag>,
    ) {
        self.send(UpdaterRequest::EntryUpdate {
            entry_id,
            tags: Some(tags),
        })
        .await;
    }

    /// Save a command's result.
    pub async fn save_command(
        &self,
        entry_id: EntryDbId,
        command: &CommandResultContext,
    ) {
        let (output, success) = match &command.result {
            CommandResult::Running => (String::new(), false),
            CommandResult::Finished { output, success } => {
                ((**output).clone(), *success)
            }
        };
        self.send(UpdaterRequest::CommandUpdate {
            entry_id,
            command: (*command.command.name).clone(),
            result: match success {
                true => 0,
                false => 1,
            },
            output,
        })
        .await;
    }
}
