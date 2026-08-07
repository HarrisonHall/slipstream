//! Slipstream task manager.
//! The task manager is responsible for running tasks in the system (e.g., database
//! operations).

use super::*;

use tokio::sync::broadcast;
use tokio::sync::oneshot;

mod commands;
mod handle;
pub mod tasks;

pub use commands::*;
pub use handle::*;
pub use tasks::*;

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

    // Spawn an infinite task so that command_futures will only wait for valid tasks.
    let handle = task_manager.handle()?;
    {
        let cancel_token = cancel_token.clone();
        task_manager.command_futures.spawn(async move {
            cancel_token.cancelled().await;
            CustomCommandResult::empty()
        });
    }

    // Continue running tasks until cancelled.
    // This select is biased, handling cancel before blocking tasks, before nonblocking
    // tasks.
    'update: loop {
        tokio::select! {
            biased;
            _ = cancel_token.cancelled() => break 'update,
            task = task_manager.to_updater_blocking_receiver.recv() => {
                if let Some(task) = task {
                    task_manager.handle_blocking_task(task, &config).await;
                }
            },
            task = task_manager.to_updater_nonblocking_receiver.recv() => match task {
                  Ok(task) => task_manager.handle_background_task(task, &config).await,
                  Err(_) => {
                      cancel_token.cancel();
                  }
            },
            completed_task = task_manager.command_futures.join_next() => {
                if let Some(Ok(completed_task)) = completed_task {
                    if completed_task.command.save {
                        handle.save_command(completed_task.entry_id.into(), completed_task).await;
                    }
                }
            },
        }
    }

    // Cleanup.
    task_manager.command_futures.abort_all();
    updater_task.abort();

    Ok(())
}

/// Standalone task for the slipfeed updater.
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
                entry_db.upsert_slipfeed_entry(entry).await;
            }
        }
    }
}

/// Slipstream updater.
pub struct TaskManager {
    /// System configuration.
    pub config: Arc<Config>,
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
    /// Futures for commands run on entries.
    command_futures: tokio::task::JoinSet<CustomCommandResult>,
    /// Handle's blocking sender.
    to_updater_blocking_sender: Sender<BlockingTask>,
    /// Updater's blocking receiver.
    to_updater_blocking_receiver: Receiver<BlockingTask>,
    /// Handle's nonblocking sender.
    to_updater_nonblocking_sender: broadcast::Sender<BackgroundTask>,
    /// Updater's nonblocking receiver.
    to_updater_nonblocking_receiver: broadcast::Receiver<BackgroundTask>,
}

impl TaskManager {
    /// Create a new task manager.
    pub fn new(config: Arc<Config>) -> Self {
        let (to_updater_blocking_sender, to_updater_blocking_receiver) =
            channel(64);
        let (to_updater_nonblocking_sender, to_updater_nonblocking_receiver) =
            broadcast::channel(64_000);
        Self {
            config,
            updater: Arc::new(RwLock::new(slipfeed::Updater::default())),
            feeds: HashMap::default(),
            feeds_ids: HashMap::default(),
            all_filters: Vec::default(),
            entry_db: None,
            command_futures: JoinSet::new(),
            to_updater_blocking_sender: to_updater_blocking_sender,
            to_updater_blocking_receiver: to_updater_blocking_receiver,
            to_updater_nonblocking_sender: to_updater_nonblocking_sender,
            to_updater_nonblocking_receiver: to_updater_nonblocking_receiver,
        }
    }

    /// Get handle to updater.
    pub fn handle(&self) -> Result<TaskManagerHandle> {
        Ok(TaskManagerHandle {
            config: self.config.clone(),
            to_updater_blocking_sender: self.to_updater_blocking_sender.clone(),
            to_updater_nonblocking_sender: self
                .to_updater_nonblocking_sender
                .clone(),
        })
    }

    /// Handle blocking task.
    async fn handle_blocking_task(
        &mut self,
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

    /// Handle background task.
    async fn handle_background_task(
        &mut self,
        task: BackgroundTask,
        _config: &Config,
    ) {
        tracing::info!(
            "REMAINING: {}",
            self.to_updater_nonblocking_receiver.len()
        );
        match task {
            BackgroundTask::Update(u) => match u {
                BackgroundTaskUpdate::EntryTagUpdate { ctx, tags } => {
                    tracing::info!("TAG");
                    if let (Some(entry_id), Some(entry_db)) =
                        (&ctx.entry_id, &self.entry_db)
                    {
                        entry_db.update_tags(*entry_id, tags).await;
                    }
                }
                BackgroundTaskUpdate::CommandUpdate { ctx: _, result } => {
                    tracing::info!("COMMAND");
                    if let Some(entry_db) = &self.entry_db {
                        entry_db
                            .store_command_result(
                                result.entry_id,
                                (*result.command.name).clone(),
                                result.output,
                                result.exit_code == 0,
                            )
                            .await;
                    }
                }
            },
            BackgroundTask::Execute(e) => match e {
                BackgroundTaskExecute::Command { ctx, commandish } => {
                    match commandish {
                        Commandish::CustomCommandRef(name) => {
                            tracing::warn!(
                                "Command ref has not been expanded: {name}"
                            );
                        }
                        Commandish::CustomCommandFull(custom_command) => {
                            if let (Some(entry_id), Ok(handle)) =
                                (ctx.entry_id, self.handle())
                            {
                                self.command_futures.spawn(run_custom_command(
                                    handle,
                                    custom_command,
                                    entry_id,
                                ));
                            }
                        }
                        Commandish::Literal(_) => {
                            // Task manager only handles select literals.
                            // match command {
                            //     ReadCommandLiteral::Command(command) => {
                            //         if self.interaction_state.selection < self.entries.len() {
                            //             if let Err(e) =
                            //                 self.handle_command_mode_command(&command).await
                            //             {
                            //                 tracing::error!("Failed to run command: {}", e);
                            //             }
                            //         }
                            //     }
                            //     _ => {},
                            // };
                        }
                    }
                }
                BackgroundTaskExecute::Hook { ctx, hook } => {
                    tracing::info!("HOOK");
                    if let (Some(commandishes), Ok(handle)) =
                        (self.config.hooks.get(&hook), self.handle())
                    {
                        for commandish in commandishes {
                            handle
                                .run_command(ctx.clone(), commandish.clone())
                                .await;
                        }
                    }
                }
            },
        }
    }

    /// Check if entry passes the all filters.
    pub fn passes_all_filters(&self, entry: &slipfeed::Entry) -> bool {
        let feed = slipfeed::NoopFeed::default();
        self.all_filters.iter().all(|f| f(&feed, entry))
    }
}
