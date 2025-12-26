mod error;
mod migration;

#[cfg(feature = "sqlite")]
mod sqlite;

pub use error::{Error, Result};
pub use migration::{SqlExecutor, SqlMigration};

#[cfg(feature = "sqlite")]
pub use sqlite::{SqliteContext, SqliteStore};

// Re-export core types for convenience
pub use lib_migrations_core::{
    FnMigration, MemoryStore, Migration, MigrationRecord, MigrationRunner, MigrationStatus,
    MigrationStore,
};
