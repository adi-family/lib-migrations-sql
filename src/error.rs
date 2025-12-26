use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("SQL execution failed: {0}")]
    Sql(String),

    #[error("Migration error: {0}")]
    Migration(#[from] lib_migrations_core::Error),

    #[cfg(feature = "sqlite")]
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

impl Error {
    pub fn sql(msg: impl Into<String>) -> Self {
        Self::Sql(msg.into())
    }
}
