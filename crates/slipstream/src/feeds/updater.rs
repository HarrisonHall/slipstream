//! Slipstream updater.

use super::*;

use futures::FutureExt;
use tokio::sync::oneshot;

/// Run the slipstream updater.
pub async fn update(
    mut updater: Updater,
    config: Arc<Config>,
    cancel_token: CancellationToken,
) -> Result<()> {
    // We don't want to cancel the updater's updater update future.
    let mut updater_fut = {
        let internal_updater = updater.updater.clone();
        async move {
            let mut updater = internal_updater.write().await;
            updater.update().await
        }
        .boxed()
    };

    // Continue updating and responding to requests until cancelled.
    'update: loop {
        tokio::select! {
            entries = &mut updater_fut => {
                updater.handle_update_entry_db(entries).await;
                updater_fut = {
                    let internal_updater = updater.updater.clone();
                    async move {
                        let mut updater = internal_updater.write().await;
                        updater.update().await
                    }
                    .boxed()
                };
            },
            command = updater.to_updater_receiver.recv() => {
                if let Some(command) = command {
                    updater.handle_command(command, &config).await;
                }
            },
            _ = cancel_token.cancelled() => break 'update,
        }
    }

    Ok(())
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
    pub entry_db: Option<Database>,
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

    /// Update entry_db with entries from updater.
    async fn handle_update_entry_db(&mut self, entries: slipfeed::EntrySet) {
        if let Some(entry_db) = &mut self.entry_db {
            for entry in entries.as_slice() {
                entry_db.insert_slipfeed_entry(entry).await;
            }
        }
    }

    /// Handle command.
    async fn handle_command(
        &mut self,
        command: UpdaterRequest,
        config: &Arc<Config>,
    ) {
        match command {
            UpdaterRequest::UpdateEntry {
                entry_id,
                important,
                read,
            } => {
                if let Some(entry_db) = &mut self.entry_db {
                    if let Some(important) = important {
                        entry_db.toggle_important(entry_id, important).await;
                    }
                    if let Some(read) = read {
                        entry_db.toggle_read(entry_id, read).await;
                    }
                }
            }
            UpdaterRequest::RequestUpdate { tx, entry } => {
                let mut entry = entry;
                if let Some(entry_db) = &mut self.entry_db {
                    entry_db.update_slipstream_entry(&mut entry).await;
                    tx.send(entry).ok();
                }
            }
            UpdaterRequest::UpdateCommand {
                entry_id,
                command,
                result,
                output,
            } => {
                if let Some(entry_db) = &mut self.entry_db {
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
            UpdaterRequest::RequestEntries { tx, config, search } => {
                match search {
                    DatabaseSearch::Latest => {
                        if let Some(entry_db) = &mut self.entry_db {
                            tx.send(
                                entry_db
                                    .get_entries(
                                        DatabaseSearch::Latest,
                                        config.global.limits.max(),
                                    )
                                    .await,
                            )
                            .ok();
                        };
                    }
                    _ => {
                        tx.send(DatabaseEntryList::new(0)).ok();
                    }
                };
            }
            UpdaterRequest::RequestSyndication { tx, config, search } => {
                todo!()
            }
            UpdaterRequest::FeedName { tx, feed } => {
                // config.feed(feed)
                tx.send(self.feeds_ids.get(&feed).map(|f| f.clone())).ok();
            }
        }
    }

    /// Get feed name from id.
    pub fn feed_name(&self, feed: slipfeed::FeedId) -> Option<&String> {
        self.feeds_ids.get(&feed)
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

    pub fn syndicate_all(&self, config: &Config) -> String {
        let mut syn = atom::FeedBuilder::default();
        syn.title("All")
            .author(atom::PersonBuilder::default().name("slipstream").build());
        let mut count = 0;
        // for entry in self.updater.iter() {
        //     if count > config.global.limits.max() {
        //         break;
        //     }
        //     if config.global.limits.too_old(entry.date()) {
        //         continue;
        //     }
        //     if !self.passes_global_filters(&entry) {
        //         continue;
        //     }
        //     if !self.passes_all_filters(&entry) {
        //         continue;
        //     }
        //     syn.entry(entry.to_atom(self, config));
        //     count += 1;
        // }
        syn.build().to_string()
    }

    pub fn collect_all(&self, config: &Config) -> Vec<slipfeed::Entry> {
        let mut entries = Vec::with_capacity(config.global.limits.max());
        let mut count = 0;
        // for entry in self.updater.iter() {
        //     if count > config.global.limits.max() {
        //         break;
        //     }
        //     if config.global.limits.too_old(entry.date()) {
        //         continue;
        //     }
        //     if !self.passes_global_filters(&entry) {
        //         continue;
        //     }
        //     if !self.passes_all_filters(&entry) {
        //         continue;
        //     }
        //     entries.push(entry.clone());
        //     count += 1;
        // }
        entries
    }

    /// Convert the feed into an atom feed.
    pub fn syndicate_feed(&self, feed: &str, config: &Config) -> String {
        let mut syn = atom::FeedBuilder::default();
        syn.title(feed)
            .author(atom::PersonBuilder::default().name("slipstream").build());
        if let (Some(id), Some(feed)) =
            (self.feeds.get(feed), config.feed(feed))
        {
            let mut count = 0;
            // for entry in self.updater.from_feed(*id) {
            //     if count >= config.global.limits.max() {
            //         break;
            //     }
            //     if count >= feed.options().max() {
            //         break;
            //     }
            //     if config.global.limits.too_old(entry.date()) {
            //         continue;
            //     }
            //     if feed.options().too_old(entry.date()) {
            //         continue;
            //     }
            //     if !self.passes_global_filters(&entry) {
            //         continue;
            //     }
            //     // NOTE: Individual feed filters are already checked by the underlying
            //     // slipfeed updater.
            //     syn.entry(entry.to_atom(self, config));
            //     count += 1;
            // }
        }
        syn.build().to_string()
    }

    /// Collect the feed.
    pub fn collect_feed(
        &self,
        feed: &str,
        config: &Config,
    ) -> Vec<slipfeed::Entry> {
        let mut entries = Vec::with_capacity(config.global.limits.max());
        if let (Some(id), Some(feed)) =
            (self.feeds.get(feed), config.feed(feed))
        {
            let mut count = 0;
            // for entry in self.updater.from_feed(*id) {
            //     if count >= config.global.limits.max() {
            //         break;
            //     }
            //     if count >= feed.options().max() {
            //         break;
            //     }
            //     if config.global.limits.too_old(entry.date()) {
            //         continue;
            //     }
            //     if feed.options().too_old(entry.date()) {
            //         continue;
            //     }
            //     if !self.passes_global_filters(&entry) {
            //         continue;
            //     }
            //     // NOTE: Individual feed filters are already checked by the underlying
            //     // slipfeed updater.
            //     entries.push(entry.clone());
            //     count += 1;
            // }
        }
        entries
    }

    /// Convert entries matching /tag to an atom feed.
    pub fn syndicate_tag(&self, tag: &str, config: &Config) -> String {
        let mut syn = atom::FeedBuilder::default();
        syn.title(tag)
            .author(atom::PersonBuilder::default().name("slipstream").build());
        let mut count = 0;
        // for entry in self.updater.with_tags(tag) {
        //     if count >= config.global.limits.max() {
        //         break;
        //     }
        //     if config.global.limits.too_old(entry.date()) {
        //         continue;
        //     }
        //     if !self.passes_global_filters(&entry) {
        //         continue;
        //     }
        //     syn.entry(entry.to_atom(self, config));
        //     count += 1;
        // }
        syn.build().to_string()
    }

    /// Collect entries matching /tag.
    pub fn collect_tag(
        &self,
        tag: &str,
        config: &Config,
    ) -> Vec<slipfeed::Entry> {
        let mut entries = Vec::with_capacity(config.global.limits.max());
        let mut count = 0;
        // for entry in self.updater.with_tags(tag) {
        //     if count >= config.global.limits.max() {
        //         break;
        //     }
        //     if config.global.limits.too_old(entry.date()) {
        //         continue;
        //     }
        //     if !self.passes_global_filters(&entry) {
        //         continue;
        //     }
        //     entries.push(entry.clone());
        //     count += 1;
        // }
        entries
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
    UpdateEntry {
        entry_id: EntryDbId,
        important: Option<bool>,
        read: Option<bool>,
    },
    RequestUpdate {
        tx: oneshot::Sender<DatabaseEntry>,
        entry: DatabaseEntry,
    },
    UpdateCommand {
        entry_id: EntryDbId,
        command: String,
        result: i32,
        output: String,
    },
    RequestEntries {
        tx: oneshot::Sender<DatabaseEntryList>,
        config: Arc<Config>,
        search: DatabaseSearch,
    },
    RequestSyndication {
        tx: oneshot::Sender<String>,
        config: Arc<Config>,
        search: DatabaseSearch,
    },
    FeedName {
        tx: oneshot::Sender<Option<String>>,
        feed: slipfeed::FeedId,
    },
    // FeedId(String),
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

    /// Collect the /all feed.
    pub async fn collect_all(&self, config: Arc<Config>) -> DatabaseEntryList {
        let (tx, rx) = oneshot::channel::<DatabaseEntryList>();
        tracing::info!("Send!");
        self.send(UpdaterRequest::RequestEntries {
            tx,
            config: config.clone(),
            search: DatabaseSearch::Latest,
        })
        .await;
        tracing::info!("Wait!");
        match rx.await {
            Ok(data) => data,
            Err(e) => {
                tracing::error!("Failed to collect_all: {}", e);
                DatabaseEntryList::new(0)
            }
        }
    }

    /// Convert the /all feed into an atom feed.
    pub async fn syndicate_all(&self, config: Arc<Config>) -> String {
        let (tx, rx) = oneshot::channel::<String>();
        self.send(UpdaterRequest::RequestSyndication {
            tx,
            config: config.clone(),
            search: DatabaseSearch::Latest,
        })
        .await;
        match rx.await {
            Ok(data) => data,
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
        config: Arc<Config>,
    ) -> DatabaseEntryList {
        let (tx, rx) = oneshot::channel::<DatabaseEntryList>();
        self.send(UpdaterRequest::RequestEntries {
            tx,
            config: config.clone(),
            search: DatabaseSearch::Feed(feed.into()),
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

    /// Convert the /feed feed into an atom feed.
    pub async fn syndicate_feed(
        &self,
        feed: impl Into<String>,
        config: Arc<Config>,
    ) -> String {
        let (tx, rx) = oneshot::channel::<String>();
        self.send(UpdaterRequest::RequestSyndication {
            tx,
            config: config.clone(),
            search: DatabaseSearch::Feed(feed.into()),
        })
        .await;
        match rx.await {
            Ok(data) => data,
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
        config: Arc<Config>,
    ) -> DatabaseEntryList {
        let (tx, rx) = oneshot::channel::<DatabaseEntryList>();
        self.send(UpdaterRequest::RequestEntries {
            tx,
            config: config.clone(),
            search: DatabaseSearch::Tag(tag.into()),
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
    ) -> String {
        let (tx, rx) = oneshot::channel::<String>();
        self.send(UpdaterRequest::RequestSyndication {
            tx,
            config: config.clone(),
            search: DatabaseSearch::Tag(tag.into()),
        })
        .await;
        match rx.await {
            Ok(data) => data,
            Err(e) => {
                tracing::error!("Failed to syndicate_tag: {}", e);
                String::new()
            }
        }
    }

    /// Get the feed name from id.
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

    pub async fn toggle_read(&self, entry_id: EntryDbId, read: bool) {
        self.send(UpdaterRequest::UpdateEntry {
            entry_id,
            important: None,
            read: Some(read),
        })
        .await;
    }

    /// Toggle the important attribute.
    pub async fn toggle_important(&self, entry_id: EntryDbId, important: bool) {
        self.send(UpdaterRequest::UpdateEntry {
            entry_id,
            important: Some(important),
            read: None,
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
        self.send(UpdaterRequest::UpdateCommand {
            entry_id,
            command: (*command.binding_name).clone(),
            result: match success {
                true => 0,
                false => 1,
            },
            output,
        })
        .await;
    }

    /// Update the view for an entry.
    pub async fn update_view(&self, entry: &mut DatabaseEntry) {
        let (tx, rx) = oneshot::channel::<DatabaseEntry>();
        self.send(UpdaterRequest::RequestUpdate {
            tx,
            entry: entry.clone(),
        })
        .await;
        match rx.await {
            Ok(data) => {
                *entry = data;
            }
            Err(e) => {
                tracing::error!("Failed to update_view: {}", e);
            }
        };
    }
}
