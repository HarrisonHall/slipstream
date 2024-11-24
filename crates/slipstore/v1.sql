-- Schema for ...

-- Feeds
CREATE TABLE IF NOT EXISTS feeds(
    id INTEGER PRIMARY KEY NOT NULL,
    type TEXT NOT NULL,
    
);

-- Entries
CREATE TABLE IF NOT EXISTS entries(
    id INTEGER PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    date INTEGER NOT NULL,
    author TEXT NOT NULL,
    content TEXT NOT NULL,
    url TEXT NOT NULL,
    tags TEXT NOT NULL  -- Serialized JSON
);
