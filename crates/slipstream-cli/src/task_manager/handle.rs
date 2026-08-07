use super::*;

/// Simple handle to manage communications with the task manager.
#[derive(Clone)]
pub struct TaskManagerHandle {
    /// System configuration.
    pub(super) config: Arc<Config>,
    /// Handle's sender.
    pub(super) to_updater_blocking_sender: Sender<BlockingTask>,
    /// Handle's sender.
    pub(super) to_updater_nonblocking_sender: broadcast::Sender<BackgroundTask>,
}

impl TaskManagerHandle {
    /// Create a receiver to handle system tasks.
    /// Once created, until dropped, the reciever is required to wait on these
    /// tasks.
    pub async fn receiver(&self) -> broadcast::Receiver<BackgroundTask> {
        self.to_updater_nonblocking_sender.subscribe()
    }

    /// Internal method to send a nonblocking task to the task manager.
    async fn send_system(&self, task: BackgroundTask) {
        let res = self.to_updater_nonblocking_sender.send(task);
        if let Err(e) = res {
            tracing::error!("Failed to send system: {}", e);
        }
    }

    /// Internal method to send a blocking task to the task manager.
    async fn send_blocking(&self, task: BlockingTask) {
        let res = self.to_updater_blocking_sender.send(task).await;
        if let Err(e) = res {
            tracing::error!("Failed to send blocking: {}", e);
        }
    }

    /// Get a single entry.
    pub async fn get_entry(&self, db_id: EntryDbId) -> Result<DatabaseEntry> {
        let (tx, rx) = oneshot::channel::<Option<DatabaseEntry>>();
        self.send_blocking(BlockingTask::EntryFetch { db_id, tx })
            .await;
        match rx.await {
            Ok(e) => match e {
                Some(e) => Ok(e),
                None => bail!("No entry {}", db_id),
            },
            Err(e) => {
                tracing::error!("Failed to search entries: {}", e);
                bail!("No entry {}", db_id);
            }
        }
    }

    /// Search for entries from a feed.
    pub async fn search(
        &self,
        criteria: Vec<DatabaseSearch>,
        offset: OffsetCursor,
    ) -> DatabaseEntryList {
        let (tx, rx) = oneshot::channel::<DatabaseEntryList>();
        self.send_blocking(BlockingTask::EntriesSearch {
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
        self.send_blocking(BlockingTask::FeedFetch {
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
        self.send_blocking(BlockingTask::FeedFetch {
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
        modified_since: Option<slipfeed::DateTime>,
    ) -> DatabaseEntryList {
        let (tx, rx) = oneshot::channel::<DatabaseEntryList>();
        self.send_blocking(BlockingTask::FeedFetch {
            tx,
            options: FeedFetchOptions::Feed {
                feed: feed.into(),
                modified_since,
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
        modified_since: Option<slipfeed::DateTime>,
    ) -> String {
        let feed = feed.into();
        let (tx, rx) = oneshot::channel::<DatabaseEntryList>();
        self.send_blocking(BlockingTask::FeedFetch {
            tx,
            options: FeedFetchOptions::Feed {
                feed: feed.clone(),
                modified_since,
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
        modified_since: Option<slipfeed::DateTime>,
    ) -> DatabaseEntryList {
        let (tx, rx) = oneshot::channel::<DatabaseEntryList>();
        self.send_blocking(BlockingTask::FeedFetch {
            tx,
            options: FeedFetchOptions::Tag {
                tag: tag.into(),
                modified_since,
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
        modified_since: Option<slipfeed::DateTime>,
    ) -> String {
        let tag = tag.into();
        let (tx, rx) = oneshot::channel::<DatabaseEntryList>();
        self.send_blocking(BlockingTask::FeedFetch {
            tx,
            options: FeedFetchOptions::Tag {
                tag: tag.clone(),
                modified_since,
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
        self.send_blocking(BlockingTask::FeedName { tx, feed: id })
            .await;
        match rx.await {
            Ok(data) => data,
            Err(e) => {
                tracing::error!("Failed to feed_name: {}", e);
                None
            }
        }
    }

    /// Update tags for an entry.
    pub async fn update_tags(
        &self,
        entry_id: EntryDbId,
        tags: Vec<slipfeed::Tag>,
    ) {
        self.send_system(
            BackgroundTaskUpdate::EntryTagUpdate {
                ctx: tasks::Context::with_entry_id(entry_id),
                tags,
            }
            .into(),
        )
        .await;
    }

    /// Save a command's result.
    pub async fn run_command(
        &self,
        ctx: tasks::Context,
        mut commandish: Commandish,
    ) {
        if let Commandish::CustomCommandRef(command_name) = &commandish {
            if let Some(expanded) =
                self.config.get_custom_command(command_name.as_str())
            {
                commandish = Commandish::CustomCommandFull(expanded);
            }
        }
        self.send_system(
            BackgroundTaskExecute::Command { ctx, commandish }.into(),
        )
        .await;
    }

    /// Save a command's result.
    pub async fn save_command(
        &self,
        ctx: tasks::Context,
        result: CustomCommandResult,
    ) {
        self.send_system(
            BackgroundTaskUpdate::CommandUpdate { ctx, result }.into(),
        )
        .await;
    }

    /// Save a command's result.
    pub async fn hook(&self, ctx: tasks::Context, hook: Hook) {
        self.send_system(BackgroundTaskExecute::Hook { ctx, hook }.into())
            .await;
    }
}
