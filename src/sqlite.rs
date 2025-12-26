use crate::migration::SqlExecutor;
use lib_migrations_core::{MigrationRecord, MigrationStore};
use rusqlite::{params, Connection};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// SQLite execution context wrapping a connection.
pub struct SqliteContext {
    conn: Connection,
}

impl SqliteContext {
    /// Open a SQLite database
    pub fn open(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA foreign_keys=ON;",
        )?;
        Ok(Self { conn })
    }

    /// Open an in-memory SQLite database
    pub fn open_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        Ok(Self { conn })
    }

    /// Get the underlying connection
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Get mutable access to the underlying connection
    pub fn connection_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Consume and return the underlying connection
    pub fn into_connection(self) -> Connection {
        self.conn
    }
}

impl SqlExecutor for SqliteContext {
    type Error = rusqlite::Error;

    fn execute(&mut self, sql: &str) -> Result<(), Self::Error> {
        self.conn.execute_batch(sql)
    }
}

/// SQLite-backed migration store.
///
/// Stores migration history in a `_migrations` table.
pub struct SqliteStore {
    conn: Connection,
}

impl SqliteStore {
    /// Open a SQLite database for migration tracking
    pub fn open(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;",
        )?;
        Ok(Self { conn })
    }

    /// Open an in-memory SQLite database
    pub fn open_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        Ok(Self { conn })
    }

    /// Get the underlying connection
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Consume and return the underlying connection
    pub fn into_connection(self) -> Connection {
        self.conn
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

impl MigrationStore for SqliteStore {
    fn init(&mut self) -> lib_migrations_core::Result<()> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS _migrations (
                    version INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    applied_at INTEGER NOT NULL
                );",
            )
            .map_err(|e| lib_migrations_core::Error::store(e.to_string()))
    }

    fn applied(&self) -> lib_migrations_core::Result<Vec<MigrationRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT version, name, applied_at FROM _migrations ORDER BY version")
            .map_err(|e| lib_migrations_core::Error::store(e.to_string()))?;

        let records = stmt
            .query_map([], |row| {
                Ok(MigrationRecord {
                    version: row.get::<_, i64>(0)? as u64,
                    name: row.get(1)?,
                    applied_at: row.get::<_, i64>(2)? as u64,
                })
            })
            .map_err(|e| lib_migrations_core::Error::store(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| lib_migrations_core::Error::store(e.to_string()))?;

        Ok(records)
    }

    fn mark_applied(&mut self, version: u64, name: &str) -> lib_migrations_core::Result<()> {
        self.conn
            .execute(
                "INSERT INTO _migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
                params![version as i64, name, Self::now() as i64],
            )
            .map_err(|e| lib_migrations_core::Error::store(e.to_string()))?;
        Ok(())
    }

    fn mark_rolled_back(&mut self, version: u64) -> lib_migrations_core::Result<()> {
        self.conn
            .execute(
                "DELETE FROM _migrations WHERE version = ?1",
                params![version as i64],
            )
            .map_err(|e| lib_migrations_core::Error::store(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SqlMigration;
    use lib_migrations_core::MigrationRunner;

    #[test]
    fn test_sqlite_store() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        store.init().unwrap();

        assert_eq!(store.current_version().unwrap(), 0);
        assert!(store.applied().unwrap().is_empty());

        store.mark_applied(1, "first").unwrap();
        assert_eq!(store.current_version().unwrap(), 1);

        let applied = store.applied().unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].version, 1);
        assert_eq!(applied[0].name, "first");

        store.mark_rolled_back(1).unwrap();
        assert_eq!(store.current_version().unwrap(), 0);
    }

    #[test]
    fn test_sqlite_context() {
        let mut ctx = SqliteContext::open_in_memory().unwrap();
        ctx.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)")
            .unwrap();
        ctx.execute("INSERT INTO test (id) VALUES (1)").unwrap();

        let count: i64 = ctx
            .connection()
            .query_row("SELECT COUNT(*) FROM test", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_full_migration_flow() {
        let store = SqliteStore::open_in_memory().unwrap();
        let mut ctx = SqliteContext::open_in_memory().unwrap();

        let mut runner = MigrationRunner::new(store)
            .add(
                SqlMigration::new(
                    1,
                    "create_users",
                    "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)",
                )
                .with_down("DROP TABLE users"),
            )
            .add(
                SqlMigration::new(
                    2,
                    "add_email",
                    "ALTER TABLE users ADD COLUMN email TEXT",
                )
                .with_down("ALTER TABLE users DROP COLUMN email"),
            );

        runner.init().unwrap();

        // Check pending
        let pending = runner.pending().unwrap();
        assert_eq!(pending.len(), 2);

        // Migrate
        let count = runner.migrate(&mut ctx).unwrap();
        assert_eq!(count, 2);
        assert_eq!(runner.current_version().unwrap(), 2);

        // Verify table exists
        ctx.execute("INSERT INTO users (name, email) VALUES ('test', 'test@example.com')")
            .unwrap();

        // Rollback to version 1
        runner.migrate_to(&mut ctx, 1).unwrap();
        assert_eq!(runner.current_version().unwrap(), 1);

        // Rollback to version 0
        runner.migrate_to(&mut ctx, 0).unwrap();
        assert_eq!(runner.current_version().unwrap(), 0);
    }
}
