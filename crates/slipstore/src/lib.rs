//! Slipstore.

use std::path::Path;

use sqlx::sqlite::{Sqlite, SqliteConnectOptions, SqlitePool, SqlitePoolOptions};

mod tests;

struct Store {
    pool: SqlitePool,
}

enum StoreError {
    Any,
}

impl From<sqlx::Error> for StoreError {
    fn from(_: sqlx::Error) -> Self {
        StoreError::Any
    }
}

impl Store {
    pub async fn new(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await?;
        Ok(Self { pool })
    }

    fn setup(self) -> Result<(), ()> {
        todo!()
    }
}
