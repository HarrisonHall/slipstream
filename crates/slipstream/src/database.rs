//! Slipfeed database.

use super::*;

use std::path::PathBuf;

use resolve_path::PathResolveExt;
use sqlx::{
    Row, SqlitePool, sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions,
};

use crate::modes::DatabaseEntry;

/// Slipfeed database abstraction.
pub struct Database {
    /// Path to the sqlite database.
    #[allow(unused)]
    path: String,
    /// Connection to the sqlite database.
    pool: SqlitePool,
}

impl Database {
    pub async fn new(path: impl AsRef<str>) -> Result<Self> {
        // Parse path and create parents if necessary.
        let path: String = match path.as_ref() {
            ":memory:" => ":memory:".into(),
            _ => {
                let mut path: PathBuf = path.as_ref().into();
                path = path.resolve().into_owned();
                if let Some(parent) = path.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        tracing::error!(
                            "Failed to create parent directory: {e}"
                        );
                    }
                }
                path.to_string_lossy().into_owned()
            }
        };

        // Create pool at path.
        tracing::info!("Using database: {}", &path);
        let pool = SqlitePoolOptions::new()
            .min_connections(2)
            .max_connections(4)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(path.clone())
                    .create_if_missing(true),
            )
            .await?;

        // Initialize database.
        Database::initialize(&pool).await?;

        Ok(Self { path, pool })
    }

    async fn initialize(pool: &SqlitePool) -> Result<()> {
        let res = sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS entries(
                id INTEGER PRIMARY KEY ASC,
                -- When the entry was created.
                timestamp INTEGER NOT NULL,
                -- The entry json blob.
                entry TEXT NOT NULL,
                -- Entry title.
                title TEXT NOT NULL,
                -- Entry content.
                content TEXT NOT NULL,
                -- Entry author.
                author TEXT NOT NULL,
                -- Entry source_id.
                -- This is the source provided by the _original_ feed.
                source_id TEXT DEFAULT NULL
            ) STRICT;
            CREATE INDEX IF NOT EXISTS entries_entry_idx ON entries(entry);
            CREATE INDEX IF NOT EXISTS entries_timestamp_idx ON entries(timestamp);
            CREATE INDEX IF NOT EXISTS entries_title_idx ON entries(title);
            CREATE INDEX IF NOT EXISTS entries_content_idx ON entries(content);
            CREATE INDEX IF NOT EXISTS entries_author_idx ON entries(author);
            CREATE INDEX IF NOT EXISTS entries_source_id_idx ON entries(source_id);

            CREATE TABLE IF NOT EXISTS sources(
                id INTEGER PRIMARY KEY ASC,
                -- The entry id.
                entry_id INTEGER REFERENCES entries(id) NOT NULL,
                -- The entry source uri.
                source TEXT NOT NULL,
                UNIQUE(entry_id, source)
            ) STRICT;
            CREATE INDEX IF NOT EXISTS sources_source_idx ON sources(source);
            CREATE INDEX IF NOT EXISTS sources_entry_id_idx ON sources(entry_id);
            
            CREATE TABLE IF NOT EXISTS tags(
                id INTEGER PRIMARY KEY ASC,
                -- The entry id.
                entry_id INTEGER REFERENCES entries(id) NOT NULL,
                -- The entry tag.
                tag TEXT NOT NULL,
                UNIQUE(entry_id, tag)
            ) STRICT;
            CREATE INDEX IF NOT EXISTS tags_tag_idx ON tags(tag);
            CREATE INDEX IF NOT EXISTS tags_entry_id_idx ON tags(entry_id);
            
            CREATE TABLE IF NOT EXISTS commands(
                id INTEGER PRIMARY KEY ASC,
                -- The entry id.
                entry_id INTEGER REFERENCES entries(id) NOT NULL,
                -- The command timestamp.
                timestamp INTEGER NOT NULL,
                -- The command ran.
                name TEXT NOT NULL,
                -- The result.
                result TEXT NOT NULL,
                -- Boolean whether the command succeeded,
                success INTEGER NOT NULL
            ) STRICT;
            CREATE INDEX IF NOT EXISTS commands_name_idx ON commands(name);
            CREATE INDEX IF NOT EXISTS commands_timestamp_idx ON commands(timestamp);
            CREATE INDEX IF NOT EXISTS commands_entry_id_idx ON commands(entry_id);
            ",
        )
        .execute(pool)
        .await;

        res?;

        Ok(())
    }

    /// This inserts an entry into the database.
    pub async fn insert_slipfeed_entry(
        &self,
        entry: &slipfeed::Entry,
    ) -> EntryDbId {
        let entry_v1 = EntryV1::from(entry);
        let serialized_entry = SerializedEntry::V1(entry_v1.clone());
        let entry_id: EntryDbId = {
            // Find existing id.
            let mut id: (Option<EntryDbId>,) = (None,);
            // Search by entry.
            if id.0.is_none() {
                id = sqlx::query_as("SELECT id FROM entries WHERE entry = ?")
                    .bind(sqlx::types::Json::from(&serialized_entry))
                    .fetch_one(&self.pool)
                    .await
                    .unwrap_or_else(|_| (None,));
            }
            // Search by title+author.
            if id.0.is_none()
                && !entry.title().is_empty()
                && !entry.author().is_empty()
            {
                id = sqlx::query_as(
                    "SELECT id FROM entries WHERE title IS ? AND author IS ?",
                )
                .bind(entry.title())
                .bind(entry.author())
                .fetch_one(&self.pool)
                .await
                .unwrap_or_else(|_| (None,));
            }
            // Search by author+source_id.
            if id.0.is_none()
                && !entry.author().is_empty()
                && !entry.source_id().is_none()
            {
                id = sqlx::query_as(
                    "SELECT id FROM entries WHERE author IS ? AND source_id IS ?",
                )
                .bind(entry.author())
                .bind(entry.source_id())
                .fetch_one(&self.pool)
                .await
                .unwrap_or_else(|_| (None,));
            }

            match id {
                (Some(id),) => {
                    tracing::debug!("Found existing {}", id);
                    id
                }
                (None,) => {
                    let id_res: Result<(Option<EntryDbId>,), _> =
                        sqlx::query_as(
                            "
                        INSERT INTO entries (timestamp, entry, title, author, content, source_id)
                        VALUES (unixepoch(?), ?, ?, ?, ?, ?)
                        RETURNING id
                        ",
                        )
                        .bind(&entry.date().to_chrono())
                        .bind(sqlx::types::Json::from(&serialized_entry))
                        .bind(entry.title())
                        .bind(entry.author())
                        .bind(entry.content())
                        .bind(entry.source_id())
                        .fetch_one(&self.pool)
                        .await;
                    match id_res {
                        Ok(maybe_id) => match maybe_id.0 {
                            Some(id) => id,
                            None => {
                                tracing::error!("Failed to insert entry");
                                return 0;
                            }
                        },
                        Err(e) => {
                            tracing::error!("Failed: {}", e);
                            return 0;
                        }
                    }
                }
            }
        };

        // Update sources.
        for feed in entry.feeds().iter() {
            let res = sqlx::query("INSERT INTO sources (entry_id, source) VALUES (?, ?) ON CONFLICT DO NOTHING")
                .bind(entry_id)
                .bind(&*feed.name)
                .execute(&self.pool).await;
            if let Err(e) = res {
                tracing::error!("Failed to insert source: {}", e);
            }
        }

        // Update tags.
        for tag in entry.tags().iter() {
            let res = sqlx::query("INSERT INTO tags (entry_id, tag) VALUES (?, ?) ON CONFLICT DO NOTHING")
                .bind(entry_id)
                .bind(String::from(tag))
                .execute(&self.pool).await;
            if let Err(e) = res {
                tracing::error!("Failed to insert tag: {}", e);
            }
        }

        return entry_id;
    }

    pub async fn get_entries(
        &self,
        params: DatabaseSearch,
        max_length: usize,
        cursor: OffsetCursor,
    ) -> DatabaseEntryList {
        let mut set = DatabaseEntryList::new(max_length);
        let search_clause: String = match params {
            DatabaseSearch::Latest => "TRUE = TRUE".into(),
            DatabaseSearch::Search(search) => {
                let search = search.to_lowercase();
                format!(
                    "entries.title LIKE '%{search}%' OR entries.author LIKE '%{search}%' OR entries.content LIKE '%{search}%'"
                )
            }
            DatabaseSearch::Tag(tag) => format!("tags.tag LIKE '%{tag}%'"),
            DatabaseSearch::Feed(feed) => {
                format!("sources.source LIKE '%{feed}%'")
            }
        };
        let cursor_clause: String = match cursor {
            OffsetCursor::Latest => "TRUE = TRUE".into(),
            OffsetCursor::Before(dt) => {
                format!("entries.timestamp < unixepoch('{}')", dt.to_iso8601())
            }
            OffsetCursor::After(dt) => {
                format!("entries.timestamp > unixepoch('{}')", dt.to_iso8601())
            }
        };
        let res = sqlx::query(&format!(
            "
            SELECT
                entries.id,
                entries.entry,
                json_group_array(sources.source) AS sources,
                json_group_array(tags.tag) AS tags,
                json_group_object(commands.name, commands.result) AS commands
            FROM entries
                LEFT JOIN sources ON entries.id = sources.entry_id
                LEFT JOIN tags ON entries.id = tags.entry_id
                LEFT JOIN commands ON entries.id = commands.entry_id
            WHERE
                {} AND {}
            GROUP BY entries.id
            ORDER BY entries.timestamp DESC, entries.id DESC
            LIMIT ?
            ",
            cursor_clause, search_clause,
        ))
        .bind(max_length as u32)
        .fetch_all(&self.pool)
        .await;
        match res {
            Ok(rows) => {
                for row in rows.iter() {
                    let id = row.get::<EntryDbId, usize>(0);

                    // Parse serialized entry.
                    let sf_entry = slipfeed::Entry::from(
                        &row.get::<sqlx::types::Json<SerializedEntry>, usize>(
                            1,
                        )
                        .0,
                    );
                    let mut entry = DatabaseEntry::new(sf_entry, id);

                    // Parse sources.
                    let sources = row.get::<sqlx::types::Json<
                        Vec<sqlx::types::Json<Option<String>>>,
                    >, usize>(2);
                    for source in sources.0 {
                        if let Some(source) = source.0 {
                            entry.entry.add_feed(slipfeed::FeedRef {
                                id: slipfeed::FeedId(0),
                                name: Arc::new(source.clone()),
                            });
                        }
                    }

                    // Parse tags.
                    let tags = row.get::<sqlx::types::Json<
                        Vec<sqlx::types::Json<Option<String>>>,
                    >, usize>(3);
                    for tag in tags.0 {
                        if let Some(tag) = tag.0 {
                            entry.entry.add_tag(&slipfeed::Tag::new(tag));
                        }
                    }

                    // Parse commands.
                    let commands = row.try_get::<sqlx::types::Json<
                        HashMap<String, String>,
                    >, usize>(4);
                    if let Ok(commands) = &commands {
                        for command in &commands.0 {
                            entry.add_result(CommandResultContext {
                                command: CustomCommand {
                                    name: Arc::new(command.0.clone()),
                                    command: Arc::new(Vec::new()),
                                    save: false,
                                },
                                result: CommandResult::Finished {
                                    output: Arc::new(command.1.clone()),
                                    success: true, // TODO!
                                },
                                vertical_scroll: 0,
                            });
                        }
                    }

                    // Parse flags.
                    // entry.has_been_read = row.get::<bool, usize>(5);
                    // entry.important = row.get::<bool, usize>(6);

                    set.add(entry).ok();
                }
            }
            Err(e) => {
                tracing::error!("Failed to get_latest_entries: {}", e);
            }
        }

        tracing::trace!("Got latest: {}", set.len());
        set
    }

    pub async fn update_tags(
        &self,
        entry_id: EntryDbId,
        tags: Vec<slipfeed::Tag>,
    ) {
        let res = sqlx::query("DELETE FROM tags WHERE entry_id = ?")
            .bind(entry_id)
            .execute(&self.pool)
            .await;
        if let Err(e) = res {
            tracing::error!("Failed to remove tags: {}", e);
        }

        for tag in &tags {
            let res =
                sqlx::query("INSERT INTO tags (entry_id, tag) VALUES(?, ?)")
                    .bind(entry_id)
                    .bind(&String::from(tag))
                    .execute(&self.pool)
                    .await;

            if let Err(e) = res {
                tracing::error!("Failed to insert tag: {}", e);
            }
        }
    }

    pub async fn store_command_result(
        &self,
        entry_id: EntryDbId,
        command: String,
        result: String,
        success: bool,
    ) {
        let res = sqlx::query("INSERT INTO commands (entry_id, name, result, success, timestamp) VALUES (?, ?, ?, ?, unixepoch(current_timestamp))")
            .bind(entry_id)
            .bind(command)
            .bind(result)
            .bind(success)
            .execute(&self.pool)
            .await;

        if let Err(e) = res {
            tracing::error!("Failed to store command result: {}", e);
        }
    }
}

/// Message used to communicate with the database handler.
#[derive(Debug, Clone)]
pub enum DatabaseSearch {
    Latest,
    Search(String),
    Tag(String),
    Feed(String),
}

/// Database identifier for entries.
pub(crate) type EntryDbId = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum SerializedEntry {
    V1(EntryV1),
}

impl From<&SerializedEntry> for slipfeed::Entry {
    fn from(value: &SerializedEntry) -> Self {
        match value {
            SerializedEntry::V1(v1) => v1.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryV1 {
    title: String,
    date: slipfeed::DateTime,
    author: String,
    content: String,
    source: slipfeed::Link,
    comments: slipfeed::Link,
    other_links: Vec<slipfeed::Link>,
}

impl From<&EntryV1> for slipfeed::Entry {
    fn from(value: &EntryV1) -> Self {
        let mut entry = slipfeed::EntryBuilder::new();
        entry
            .title(&value.title)
            .date(value.date.clone())
            .author(&value.author)
            .content(&value.content)
            .source(&value.source.url)
            .comments(&value.comments.url);
        for link in &value.other_links {
            entry.other_link(link.clone());
        }
        entry.build()
    }
}

impl From<&slipfeed::Entry> for EntryV1 {
    fn from(value: &slipfeed::Entry) -> Self {
        EntryV1 {
            title: value.title().clone(),
            date: value.date().clone(),
            author: value.author().clone(),
            content: value.content().clone(),
            source: value.source().clone(),
            comments: value.comments().clone(),
            other_links: value.other_links().clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum OffsetCursor {
    Latest,
    Before(slipfeed::DateTime),
    After(slipfeed::DateTime),
}
