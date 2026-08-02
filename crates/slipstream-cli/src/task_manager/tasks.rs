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

/// Nonblocking tasks.
#[derive(Debug, Clone)]
pub enum NonblockingTask {
    EntryTagUpdate {
        entry_id: EntryDbId,
        tags: Option<Vec<slipfeed::Tag>>,
    },
    CommandUpdate(CustomCommandResult),
    RunCustomCommand {
        entry_id: EntryDbId,
        command: CustomCommand,
    },
}

#[derive(Debug, Clone)]
pub enum SystemTask {
    TaskManager(NonblockingTask),
    RunCommand {
        entry_id: Option<EntryDbId>,
        commandish: Commandish,
    },
    RunHook {
        entry_id: Option<EntryDbId>,
        hook: Hook,
    },
}
