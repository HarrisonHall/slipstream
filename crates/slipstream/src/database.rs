//! Slipfeed database.

use super::*;

use std::path::PathBuf;

use resolve_path::PathResolveExt;
use sqlx::{Connection, Row, SqliteConnection, sqlite::SqliteConnectOptions};

use crate::modes::DatabaseEntry;

/// Slipfeed database abstraction.
pub struct Database {
    /// Path to the sqlite database.
    path: String,
    /// Connection to the sqlite database.
    conn: sqlx::SqliteConnection,
}

impl Database {
    pub async fn new(path: impl AsRef<str>) -> Result<Self> {
        let path: String = match path.as_ref() {
            ":memory:" => ":memory:".into(),
            _ => {
                let mut path: PathBuf = path.as_ref().into();
                path = path.resolve().into_owned();
                // TODO: Create parent directory.
                path.to_string_lossy().into_owned()
            }
        };
        tracing::info!("Using database: {}", &path);
        let options = SqliteConnectOptions::new()
            .filename(path.clone())
            .create_if_missing(true);
        let mut conn = SqliteConnection::connect_with(&options).await.unwrap();
        Database::initialize(&mut conn).await?;
        Ok(Self { path, conn })
    }

    async fn initialize(conn: &mut sqlx::SqliteConnection) -> Result<()> {
        let res = sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS entries(
                id INTEGER PRIMARY KEY ASC,
                -- When the entry was created.
                timestamp INTEGER NOT NULL,
                -- The entry json blob.
                entry TEXT NOT NULL,
                -- If the entry is marked important.
                important INTEGER NOT NULL DEFAULT FALSE,
                -- If the entry has been read.
                read INTEGER NOT NULL DEFAULT FALSE,
                UNIQUE(entry)
            ) STRICT;
            CREATE INDEX IF NOT EXISTS entries_entry_idx ON entries(entry);
            CREATE INDEX IF NOT EXISTS entries_timestamp_idx ON entries(timestamp);

            CREATE TABLE IF NOT EXISTS sources(
                id INTEGER PRIMARY KEY ASC,
                -- The entry id.
                entry_id INTEGER REFERENCES entries(id) NOT NULL,
                -- The entry source uri.
                source TEXT NOT NULL,
                UNIQUE(entry_id, source)
            ) STRICT;
            CREATE INDEX IF NOT EXISTS sources_source_idx ON sources(source);
            
            CREATE TABLE IF NOT EXISTS tags(
                id INTEGER PRIMARY KEY ASC,
                -- The entry id.
                entry_id INTEGER REFERENCES entries(id) NOT NULL,
                -- The entry tag.
                tag TEXT NOT NULL,
                UNIQUE(entry_id, tag)
            ) STRICT;
            CREATE INDEX IF NOT EXISTS tags_tag_idx ON tags(tag);
            
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
            ",
        )
        .execute(conn)
        .await;

        res?;

        Ok(())
    }

    /// This inserts an entry into the database.
    pub async fn insert_slipfeed_entry(
        &mut self,
        entry: &slipfeed::Entry,
    ) -> EntryDbId {
        let entry_v1 = EntryV1::from(entry);
        let serialized_entry = SerializedEntry::V1(entry_v1.clone());
        let entry_id: EntryDbId = {
            let id: (Option<EntryDbId>,) =
                sqlx::query_as("SELECT id, read, important FROM entries WHERE entry = ? OR (FALSE)")
                    .bind(sqlx::types::Json::from(&serialized_entry))
                    .fetch_one(&mut self.conn)
                    .await
                    .unwrap_or_else(|_| (None,));
            match id {
                (Some(id),) => {
                    tracing::debug!("Found existing {}", id);
                    id
                }
                (None,) => {
                    let now = slipfeed::DateTime::now().to_chrono();
                    let id_res: Result<(Option<EntryDbId>,), _> =
                        sqlx::query_as(
                            "
                        INSERT INTO entries (timestamp, entry)
                        VALUES (?, ?)
                        RETURNING id
                        ",
                        )
                        .bind(&now.timestamp())
                        .bind(sqlx::types::Json::from(&serialized_entry))
                        .fetch_one(&mut self.conn)
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
                .execute(&mut self.conn).await;
            if let Err(e) = res {
                tracing::error!("Failed to insert source: {}", e);
            }
        }

        // Update tags.
        for tag in entry.tags().iter() {
            let res = sqlx::query("INSERT INTO tags (entry_id, tag) VALUES (?, ?) ON CONFLICT DO NOTHING")
                .bind(entry_id)
                .bind(String::from(tag))
                .execute(&mut self.conn).await;
            if let Err(e) = res {
                tracing::error!("Failed to insert tag: {}", e);
            }
        }

        return entry_id;
    }

    pub async fn update_slipstream_entry(&mut self, entry: &mut DatabaseEntry) {
        let data: (Option<bool>, Option<bool>) = sqlx::query_as(
            "SELECT read, important FROM entries WHERE id = ? LIMIT 1",
        )
        .bind(entry.db_id)
        .fetch_one(&mut self.conn)
        .await
        .unwrap_or_else(|_| (None, None));
        match data {
            (Some(read), Some(important)) => {
                entry.has_been_read = read;
                entry.important = important;
            }
            _ => {
                tracing::error!("Failed to find entry {}", entry.db_id);
                entry.has_been_read = false;
                entry.important = false;
            }
        }
    }

    pub async fn get_entries(
        &mut self,
        _params: DatabaseSearch,
        max_length: usize,
    ) -> DatabaseEntryList {
        let mut set = DatabaseEntryList::new(max_length);
        let res = sqlx::query(
            "
            SELECT
                entries.id,
                entries.entry,
                json_group_array(sources.source) AS sources,
                json_group_array(tags.tag) AS tags,
                json_group_object(commands.name, commands.result) AS commands,
                entries.read,
                entries.important
            FROM entries
                LEFT JOIN sources ON entries.id = sources.entry_id
                LEFT JOIN tags ON entries.id = tags.entry_id
                LEFT JOIN commands ON entries.id = commands.entry_id
            GROUP BY entries.id
            ORDER BY entries.timestamp DESC, entries.id DESC
            LIMIT ?
            ",
        )
        .bind(max_length as u32)
        .fetch_all(&mut self.conn)
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
                    tracing::error!("HMMM {:?}", row.column(4));
                    // let commands = row.get::<Option<
                    //     sqlx::types::Json<HashMap<String, String>>,
                    // >, usize>(4);
                    // if let Some(commands) = commands {
                    //     for command in commands.0 {
                    //         entry.add_result(CommandResultContext {
                    //             binding_name: Arc::new(command.0.clone()),
                    //             result: CommandResult::Finished {
                    //                 output: Arc::new(command.1.clone()),
                    //                 success: true, // TODO!
                    //             },
                    //             vertical_scroll: 0,
                    //         });
                    //     }
                    // }
                    let commands = row.try_get::<sqlx::types::Json<
                        HashMap<String, String>,
                    >, usize>(4);
                    if let Ok(commands) = &commands {
                        tracing::error!("SUCCESS!!!");
                        for command in &commands.0 {
                            entry.add_result(CommandResultContext {
                                binding_name: Arc::new(command.0.clone()),
                                result: CommandResult::Finished {
                                    output: Arc::new(command.1.clone()),
                                    success: true, // TODO!
                                },
                                vertical_scroll: 0,
                            });
                        }
                    } else {
                        // tracing::error!("hmmm: {:?}", commands);
                    }

                    // Parse flags.
                    entry.has_been_read = row.get::<bool, usize>(5);
                    entry.important = row.get::<bool, usize>(6);

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

    pub async fn toggle_important(
        &mut self,
        entry_id: EntryDbId,
        important: bool,
    ) {
        let res = sqlx::query("UPDATE entries SET important = ? WHERE id = ?")
            .bind(important)
            .bind(entry_id)
            .execute(&mut self.conn)
            .await;

        if let Err(e) = res {
            tracing::error!("Failed to toggle important: {}", e);
        }
    }

    pub async fn toggle_read(&mut self, entry_id: EntryDbId, read: bool) {
        let res = sqlx::query("UPDATE entries SET read = ? WHERE id = ?")
            .bind(read)
            .bind(entry_id)
            .execute(&mut self.conn)
            .await;

        if let Err(e) = res {
            tracing::error!("Failed to toggle read: {}", e);
        }
    }

    pub async fn store_command_result(
        &mut self,
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
            .execute(&mut self.conn)
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
    // feeds? TODO
    tags: Vec<slipfeed::Tag>,
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
        let mut entry = entry.build();
        for tag in &value.tags {
            entry.add_tag(tag);
        }
        entry
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
            tags: value.tags().iter().map(|tag| tag.clone()).collect(),
        }
    }
}

// struct DatabaseCommandJson {

// }
