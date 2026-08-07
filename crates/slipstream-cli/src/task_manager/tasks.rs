//! Task manager tasks & execution.

use super::*;

/// Blocking tasks.
/// Tasks that return results.
/// These tasks may be higher priority and may block other operations by slipstream.
#[derive(Debug)]
pub enum BlockingTask {
    /// Fetch an entry.
    EntryFetch {
        tx: oneshot::Sender<Option<DatabaseEntry>>,
        db_id: EntryDbId,
    },
    /// Custom search.
    EntriesSearch {
        tx: oneshot::Sender<DatabaseEntryList>,
        criteria: Vec<DatabaseSearch>,
        offset: OffsetCursor,
    },
    /// Search for singel feed.
    FeedFetch {
        tx: oneshot::Sender<DatabaseEntryList>,
        options: FeedFetchOptions,
    },
    /// Get the feed name for an id.
    FeedName {
        tx: oneshot::Sender<Option<String>>,
        feed: slipfeed::FeedId,
    },
}

/// Feed search parameters.
#[derive(Debug, Clone)]
pub enum FeedFetchOptions {
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

/// Background tasks.
#[derive(Debug, Clone)]
pub enum BackgroundTask {
    /// Update task.
    Update(BackgroundTaskUpdate),
    /// Execute task.
    Execute(BackgroundTaskExecute),
}

#[derive(Debug, Clone)]
pub enum BackgroundTaskUpdate {
    EntryTagUpdate {
        ctx: Context,
        tags: Vec<slipfeed::Tag>,
    },
    CommandUpdate {
        ctx: Context,
        result: CustomCommandResult,
    },
}

impl From<BackgroundTaskUpdate> for BackgroundTask {
    fn from(value: BackgroundTaskUpdate) -> Self {
        Self::Update(value)
    }
}

#[derive(Debug, Clone)]
pub enum BackgroundTaskExecute {
    Command {
        ctx: Context,
        commandish: Commandish,
    },
    Hook {
        ctx: Context,
        hook: Hook,
    },
}

impl From<BackgroundTaskExecute> for BackgroundTask {
    fn from(value: BackgroundTaskExecute) -> Self {
        Self::Execute(value)
    }
}

/// Task execution context.
#[derive(Debug, Default, Clone)]
pub struct Context {
    /// Associated entry id.
    pub entry_id: Option<EntryDbId>,
    /// Associated feed reference.
    pub feed_ref: Option<slipfeed::FeedRef>,
}

impl Context {
    pub fn with_entry_id(entry_id: EntryDbId) -> Self {
        Self {
            entry_id: Some(entry_id),
            ..Default::default()
        }
    }

    pub fn with_feed_ref(feed_ref: slipfeed::FeedRef) -> Self {
        Self {
            feed_ref: Some(feed_ref),
            ..Default::default()
        }
    }
}

impl From<EntryDbId> for Context {
    fn from(value: EntryDbId) -> Self {
        Self::with_entry_id(value)
    }
}

impl From<Option<EntryDbId>> for Context {
    fn from(value: Option<EntryDbId>) -> Self {
        match &value {
            Some(v) => (*v).into(),
            None => Self::default(),
        }
    }
}

impl From<slipfeed::FeedRef> for Context {
    fn from(value: slipfeed::FeedRef) -> Self {
        Self::with_feed_ref(value)
    }
}

/// Helper class to build a task execution context.
#[derive(Debug, Default, Clone)]
pub struct ContextBuilder {
    task_execution: Context,
}

impl ContextBuilder {
    /// Build.
    pub fn build(self) -> Context {
        self.task_execution
    }

    /// Set the entry id.
    pub fn entry_id(mut self, entry_id: EntryDbId) -> Self {
        self.task_execution.entry_id = Some(entry_id);
        self
    }

    /// Set the feed ref.
    pub fn feed_ref(mut self, feed_ref: slipfeed::FeedRef) -> Self {
        self.task_execution.feed_ref = Some(feed_ref);
        self
    }
}
