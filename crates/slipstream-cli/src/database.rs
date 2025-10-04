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
    /// Path to the sqlite database file.
    /// This is ":memory:" if the database is unspecified.
    #[allow(unused)]
    path: String,
    /// Connection to the sqlite database.
    pool: SqlitePool,
}

impl Database {
    /// Create a new database.
    pub async fn new(path: impl AsRef<str>) -> Result<Self> {
        // Parse path and create parents if necessary. Additionally set connect
        // options according to the specified path.
        let options: SqliteConnectOptions;
        let path: String = match path.as_ref() {
            ":memory:" => {
                options = SqliteConnectOptions::from_str(":memory:")?;
                ":memory:".into()
            }
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
                let path = path.to_string_lossy().into_owned();
                options = SqliteConnectOptions::new()
                    .filename(path.clone())
                    .create_if_missing(true);
                path
            }
        };

        // Create pool at path.
        tracing::debug!("Using database: {}", &path);
        let pool = SqlitePoolOptions::new()
            .min_connections(2)
            .max_connections(4)
            .connect_with(options)
            .await?;

        // Initialize database.
        Database::initialize(&pool).await?;

        Ok(Self { path, pool })
    }

    /// Initialize the database.
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

        if let Err(e) = res {
            bail!("Failed to initialize database: {e}");
        }

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
                    tracing::trace!(
                        "No insertion, found existing entry {}.",
                        id
                    );
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
                            Some(id) => {
                                tracing::trace!("Insertion, new entry {}.", id);
                                id
                            }
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
        criteria: Vec<DatabaseSearch>,
        max_length: usize,
        cursor: OffsetCursor,
    ) -> DatabaseEntryList {
        let mut set = DatabaseEntryList::new(max_length);
        let mut search_clause = vec!["TRUE = TRUE".to_string()];
        let mut order_clause =
            String::from("ORDER BY entries.timestamp DESC, entries.id DESC");
        for crit in &criteria {
            match crit {
                DatabaseSearch::Latest => {}
                DatabaseSearch::Live => {
                    order_clause = "ORDER BY entries.id DESC".into();
                }
                DatabaseSearch::Search(search) => {
                    let search = search.to_lowercase();
                    search_clause.push(format!(
                        "entries.title LIKE '%{search}%' OR entries.author LIKE '%{search}%' OR entries.content LIKE '%{search}%'"
                    ));
                }
                DatabaseSearch::Tag(tag) => {
                    search_clause.push(format!("tags.tag LIKE '%{tag}%'"));
                }
                DatabaseSearch::Feed(feed) => {
                    search_clause
                        .push(format!("sources.source LIKE '%{feed}%'"));
                }
                DatabaseSearch::Command(command) => {
                    search_clause
                        .push(format!("commands.name LIKE '%{command}%'"));
                }
                DatabaseSearch::Raw(raw_clause) => {
                    search_clause.push(raw_clause.clone());
                }
            };
        }
        let cursor_clause: String = match cursor {
            OffsetCursor::LatestTimestamp => "TRUE = TRUE".into(),
            OffsetCursor::LatestId => "TRUE = TRUE".into(),
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
            {}
            LIMIT ?
            ",
            cursor_clause,
            search_clause.join(" AND "),
            order_clause,
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
                                // TODO: Get the accurate feed id, if it still exists.
                                id: slipfeed::FeedId::new(0),
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
    Live,
    Search(String),
    Tag(String),
    Feed(String),
    Command(String),
    Raw(String),
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
    LatestTimestamp,
    LatestId,
    Before(slipfeed::DateTime),
    After(slipfeed::DateTime),
}

impl From<Option<slipfeed::DateTime>> for OffsetCursor {
    fn from(value: Option<slipfeed::DateTime>) -> Self {
        match value {
            Some(dt) => OffsetCursor::After(dt.clone()),
            None => OffsetCursor::LatestTimestamp,
        }
    }
}
