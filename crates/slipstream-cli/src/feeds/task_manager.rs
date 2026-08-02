//! Slipstream task manager..

use super::*;

use tokio::sync::oneshot;

/// Run the slipstream task manager.
pub async fn manage_tasks(
    mut task_manager: TaskManager,
    config: Arc<Config>,
    cancel_token: CancellationToken,
) -> Result<()> {
    // We can't cancel the task managers's updater update future as it is not cancel-safe.
    // We convert this loop into a task and only cancel on quit.
    let updater_task: tokio::task::JoinHandle<()> = {
        let entry_db = task_manager.entry_db.clone();
        let updater = task_manager.updater.clone();
        let cancel_token = cancel_token.clone();
        tokio::task::spawn(run_slipfeed_updater(
            updater,
            entry_db,
            cancel_token,
        ))
    };

    // Continue running tasks until cancelled.
    // This select is biased, handling cancel before blocking tasks, before nonblocking
    // tasks.
    'update: loop {
        tokio::select! {
            biased;
            _ = cancel_token.cancelled() => break 'update,
            task = task_manager.to_updater_blocking_receiver.recv() => {
                if let Some(task) = task {
                    task_manager.handle_blocking_command(task, &config).await;
                }
            },
            task = task_manager.to_updater_nonblocking_receiver.recv() => {
                if let Some(task) = task {
                    task_manager.handle_nonblocking_command(task, &config).await;
                }
            },
        }
    }

    updater_task.abort();

    Ok(())
}

async fn run_slipfeed_updater(
    internal_updater: Arc<RwLock<slipfeed::Updater>>,
    entry_db: Option<Arc<Database>>,
    cancel_token: CancellationToken,
) {
    while !cancel_token.is_cancelled() {
        let entries = {
            let mut slipfeed_updater = internal_updater.write().await;
            slipfeed_updater.update_blocking().await
        };
        for entry in entries.as_slice() {
            if let Some(entry_db) = &entry_db {
                entry_db.insert_slipfeed_entry(entry).await;
            }
        }
    }
}

/// Slipstream updater.
pub struct TaskManager {
    /// Underlying slipfeed updater.
    pub updater: Arc<RwLock<slipfeed::Updater>>,
    /// Map feeds by name to slipfeed id.
    pub feeds: HashMap<String, slipfeed::FeedId>,
    /// Map slipfeed ids to the feed names.
    pub feeds_ids: HashMap<slipfeed::FeedId, String>,
    /// All filters (applies to the /all feed).
    pub all_filters: Vec<slipfeed::Filter>,
    /// The entry database.
    /// This allows persistance between slipstream sessions.
    pub entry_db: Option<Arc<Database>>,
    /// Handle's blocking sender.
    to_updater_blocking_sender: Sender<BlockingTask>,
    /// Updater's blocking receiver.
    to_updater_blocking_receiver: Receiver<BlockingTask>,
    /// Handle's nonblocking sender.
    to_updater_nonblocking_sender: Sender<NonblockingTask>,
    /// Updater's nonblocking receiver.
    to_updater_nonblocking_receiver: Receiver<NonblockingTask>,
}

impl TaskManager {
    /// Get handle to updater.
    pub fn handle(&mut self) -> Result<TaskManagerHandle> {
        Ok(TaskManagerHandle {
            to_updater_blocking_sender: self.to_updater_blocking_sender.clone(),
            to_updater_nonblocking_sender: self
                .to_updater_nonblocking_sender
                .clone(),
        })
    }

    /// Handle blocking command.
    async fn handle_blocking_command(
        &self,
        task: BlockingTask,
        config: &Arc<Config>,
    ) {
        match task {
            BlockingTask::EntryFetch { tx, db_id } => {
                if let Some(entry_db) = &self.entry_db {
                    tx.send(entry_db.get_entry(db_id).await).ok();
                };
            }
            BlockingTask::EntriesSearch {
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
            BlockingTask::FeedFetch { tx, options } => {
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
                                            OffsetCursor::After(since)
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
                                if !self.passes_all_filters(entry) {
                                    continue;
                                }
                                entries.add(entry.clone()).ok();
                            }
                            entries
                        }
                        FeedFetchOptions::Tag {
                            tag,
                            modified_since,
                        } => {
                            let unfiltered_entries = entry_db
                                .get_entries(
                                    vec![DatabaseSearch::Tag(tag)],
                                    config.global.limits.max(),
                                    OffsetCursor::modified_since(
                                        modified_since,
                                    ),
                                )
                                .await;
                            let mut entries = DatabaseEntryList::new(
                                config.global.limits.max(),
                            );
                            for entry in unfiltered_entries.iter() {
                                if config.global.limits.too_old(entry.date()) {
                                    continue;
                                }
                                entries.add(entry.clone()).ok();
                            }
                            entries
                        }
                        FeedFetchOptions::Feed {
                            feed,
                            modified_since,
                        } => {
                            if let (Some(_feed_id), Some(feed_def)) =
                                (self.feeds.get(&feed), config.feed(&feed))
                            {
                                let unfiltered_entries = entry_db
                                    .get_entries(
                                        vec![DatabaseSearch::Feed(
                                            feed.clone(),
                                        )],
                                        config.global.limits.max(),
                                        OffsetCursor::modified_since(
                                            modified_since,
                                        ),
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
            BlockingTask::FeedName { tx, feed } => {
                // config.feed(feed)
                tx.send(self.feeds_ids.get(&feed).cloned()).ok();
            }
        }
    }

    /// Handle nonblocking command.
    async fn handle_nonblocking_command(
        &self,
        task: NonblockingTask,
        _config: &Arc<Config>,
    ) {
        match task {
            NonblockingTask::EntryUpdate { entry_id, tags } => {
                if let Some(entry_db) = &self.entry_db {
                    if let Some(tags) = tags {
                        entry_db.update_tags(entry_id, tags).await;
                    }
                }
            }
            NonblockingTask::CommandUpdate {
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
        }
    }

    /// Check if entry passes the all filters.
    pub fn passes_all_filters(&self, entry: &slipfeed::Entry) -> bool {
        let feed = slipfeed::NoopFeed::default();
        self.all_filters.iter().all(|f| f(&feed, entry))
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        let (to_updater_blocking_sender, to_updater_blocking_receiver) =
            channel(32);
        let (to_updater_nonblocking_sender, to_updater_nonblocking_receiver) =
            channel(1024);
        Self {
            updater: Arc::new(RwLock::new(slipfeed::Updater::default())),
            feeds: HashMap::default(),
            feeds_ids: HashMap::default(),
            all_filters: Vec::default(),
            entry_db: None,
            to_updater_blocking_sender: to_updater_blocking_sender,
            to_updater_blocking_receiver: to_updater_blocking_receiver,
            to_updater_nonblocking_sender: to_updater_nonblocking_sender,
            to_updater_nonblocking_receiver: to_updater_nonblocking_receiver,
        }
    }
}

/// Nonblocking tasks.
#[derive(Debug)]
enum NonblockingTask {
    EntryUpdate {
        entry_id: EntryDbId,
        tags: Option<Vec<slipfeed::Tag>>,
    },
    CommandUpdate {
        entry_id: EntryDbId,
        command: String,
        result: i32,
        output: String,
    },
}

/// Blocking tasks.
#[derive(Debug)]
enum BlockingTask {
    EntryFetch {
        tx: oneshot::Sender<Option<DatabaseEntry>>,
        db_id: EntryDbId,
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
}

#[derive(Debug, Clone)]
enum FeedFetchOptions {
    All {
        since: Option<slipfeed::DateTime>,
        modified_since: Option<slipfeed::DateTime>,
    },
    Feed {
        feed: String,
        modified_since: Option<slipfeed::DateTime>,
    },
    Tag {
        tag: String,
        modified_since: Option<slipfeed::DateTime>,
    },
}

/// Simple handle to manage communications with the task manager.
#[derive(Clone)]
pub struct TaskManagerHandle {
    /// Handle's sender.
    to_updater_blocking_sender: Sender<BlockingTask>,
    /// Handle's sender.
    to_updater_nonblocking_sender: Sender<NonblockingTask>,
}

impl TaskManagerHandle {
    /// Internal method to send a nonblocking task to the task manager.
    async fn send_nonblocking(&self, message: NonblockingTask) {
        let res = self.to_updater_nonblocking_sender.send(message).await;
        if let Err(e) = res {
            tracing::error!("Failed to send nonblocking: {}", e);
        }
    }

    /// Internal method to send a blocking task to the task manager.
    async fn send_blocking(&self, message: BlockingTask) {
        let res = self.to_updater_blocking_sender.send(message).await;
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
        self.send_nonblocking(NonblockingTask::EntryUpdate {
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
        self.send_nonblocking(NonblockingTask::CommandUpdate {
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

    /// Run a custom command.
    pub async fn run_custom_command(
        &self,
        custom_command: &CustomCommand,
        ctx: &CustomCommandContext,
    ) -> CustomCommandResult {
        // Get entry.
        let entry = match self.get_entry(ctx.entry_id).await {
            Ok(entry) => entry,
            Err(e) => {
                tracing::error!("Could not run command: {e}");
                return CustomCommandResult {
                    output: format!("{e}"),
                    exit_code: 1,
                };
            }
        };

        // Build command.
        let mut shell_command: Vec<String> = (*custom_command.command).clone();

        for argument in shell_command.iter_mut() {
            // Add links.
            *argument = argument.replace("{{link.url}}", &entry.source().url);
            let mut link_count: usize = 0;
            if !entry.source().url.is_empty() {
                link_count += 1;
                *argument = argument.replace(
                    &format!("{{{{link.url{}}}}}", link_count),
                    &entry.source().url,
                );
            }
            if !entry.comments().url.is_empty() {
                link_count += 1;
                *argument = argument.replace(
                    &format!("{{{{link.url{}}}}}", link_count),
                    &entry.comments().url,
                );
            }
            for i in 0..entry.other_links().len() {
                link_count += 1;
                *argument = argument.replace(
                    &format!("{{{{link.url{}}}}}", link_count),
                    &entry.other_links()[i].url,
                );
            }

            // Add link name.
            if argument.contains("{{link.name}}")
                || argument.contains("{{link.name_}}")
            {
                let link_name = entry
                    .title()
                    .clone()
                    .replace(
                        &['(', ')', ',', '\"', '.', ';', ':', '\''][..],
                        "",
                    )
                    .replace(" ", "_")
                    .to_lowercase();
                *argument = argument.replace("{{link.name}}", &link_name);
                *argument = argument.replace("{{link.name_}}", &link_name);
            }
            if argument.contains("{{link.name-}}") {
                let link_name = entry
                    .title()
                    .clone()
                    .replace(
                        &['(', ')', ',', '\"', '.', ';', ':', '\''][..],
                        "",
                    )
                    .replace(" ", "-")
                    .to_lowercase();
                *argument = argument.replace("{{link.name-}}", &link_name);
            }

            // Add feed information.
            *argument =
                argument.replace("{{feed}}", &entry.entry.primary_feed().name);

            // Add terminal settings.
            *argument = argument.replace(
                "{{terminal.width}}",
                &format!("{}", ctx.terminal_size.0),
            );
        }

        // Log final command.
        tracing::trace!("Command: {:?}", &shell_command);

        // Build subprocess.
        let mut subproc = tokio::process::Command::new(&shell_command[0]);
        subproc.args(&shell_command[1..]);

        // Run subprocess.
        let result = match subproc.output().await {
            Ok(output) => {
                let exit_code: i32 = output.status.code().unwrap_or(1);
                let output: String = match exit_code {
                    0 => String::from_utf8(output.stdout)
                        .unwrap_or_else(|_| String::new()),
                    _ => {
                        String::from_utf8(output.stderr).unwrap_or_else(|_| {
                            format!(
                                "Failed to execute command: {:?}",
                                shell_command
                            )
                        })
                    }
                };
                tracing::info!("Command:\n{:?}", &custom_command.command);
                tracing::info!("Output:\n{}", output);
                CustomCommandResult { output, exit_code }
            }
            Err(e) => CustomCommandResult {
                output: format!("Failed to run command: {}", e),
                exit_code: 1,
            },
        };

        // Store result.
        self.send_nonblocking(NonblockingTask::CommandUpdate {
            entry_id: ctx.entry_id,
            command: (*custom_command.name).clone(),
            result: result.exit_code,
            output: result.output.clone(),
        })
        .await;

        result
    }
}

/// Context required to run a custom command.
pub struct CustomCommandContext {
    /// ID of entry in db.
    pub entry_id: EntryDbId,
    /// (width, height).
    pub terminal_size: (u16, u16),
}

impl Default for CustomCommandContext {
    fn default() -> Self {
        Self {
            entry_id: 0,
            terminal_size: (80, 60),
        }
    }
}

/// Result from a custom command.
pub struct CustomCommandResult {
    /// stdout output from command.
    pub output: String,
    /// Exit code from command.
    pub exit_code: i32,
}
