use lib_migrations_core::{Migration, Phase};

/// Trait for SQL execution contexts.
///
/// Implement this for your database connection type.
pub trait SqlExecutor {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Execute SQL statement(s)
    fn execute(&mut self, sql: &str) -> std::result::Result<(), Self::Error>;
}

/// A migration defined by SQL strings.
///
/// Works with any context that implements `SqlExecutor`.
pub struct SqlMigration {
    version: u64,
    name: String,
    phase: Phase,
    up_sql: String,
    down_sql: Option<String>,
}

impl SqlMigration {
    /// Create a new SQL migration
    pub fn new(version: u64, name: impl Into<String>, up_sql: impl Into<String>) -> Self {
        Self {
            version,
            name: name.into(),
            phase: Phase::PreDeploy,
            up_sql: up_sql.into(),
            down_sql: None,
        }
    }

    /// Set the deployment phase
    pub fn phase(mut self, phase: Phase) -> Self {
        self.phase = phase;
        self
    }

    /// Add rollback SQL
    pub fn with_down(mut self, down_sql: impl Into<String>) -> Self {
        self.down_sql = Some(down_sql.into());
        self
    }

    /// Get the up SQL
    pub fn up_sql(&self) -> &str {
        &self.up_sql
    }

    /// Get the down SQL
    pub fn down_sql(&self) -> Option<&str> {
        self.down_sql.as_deref()
    }

    /// Get the version
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Get the name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the deployment phase
    pub fn get_phase(&self) -> Phase {
        self.phase
    }

    /// Whether this migration has rollback SQL
    pub fn has_rollback(&self) -> bool {
        self.down_sql.is_some()
    }
}

impl<Ctx> Migration<Ctx> for SqlMigration
where
    Ctx: SqlExecutor,
{
    fn version(&self) -> u64 {
        self.version
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn phase(&self) -> Phase {
        self.phase
    }

    fn apply(&self, ctx: &mut Ctx) -> lib_migrations_core::Result<()> {
        ctx.execute(&self.up_sql)
            .map_err(|e| lib_migrations_core::Error::failed(self.version, e.to_string()))
    }

    fn rollback(&self, ctx: &mut Ctx) -> lib_migrations_core::Result<()> {
        match &self.down_sql {
            Some(sql) => ctx
                .execute(sql)
                .map_err(|e| lib_migrations_core::Error::failed(self.version, e.to_string())),
            None => Err(lib_migrations_core::Error::RollbackNotSupported(
                self.version,
            )),
        }
    }

    fn can_rollback(&self) -> bool {
        self.down_sql.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_migration() {
        let migration = SqlMigration::new(1, "create_users", "CREATE TABLE users (id INTEGER)")
            .with_down("DROP TABLE users");

        assert_eq!(migration.version(), 1);
        assert_eq!(migration.name(), "create_users");
        assert_eq!(migration.up_sql(), "CREATE TABLE users (id INTEGER)");
        assert_eq!(migration.down_sql(), Some("DROP TABLE users"));
        assert!(migration.has_rollback());
    }

    #[test]
    fn test_sql_migration_no_rollback() {
        let migration = SqlMigration::new(1, "create_users", "CREATE TABLE users (id INTEGER)");

        assert!(!migration.has_rollback());
        assert_eq!(migration.down_sql(), None);
    }

    #[test]
    fn test_sql_migration_phase() {
        let pre = SqlMigration::new(1, "add_column", "ALTER TABLE users ADD email TEXT");
        assert_eq!(pre.get_phase(), Phase::PreDeploy);

        let post = SqlMigration::new(2, "drop_column", "ALTER TABLE users DROP old_column")
            .phase(Phase::PostDeploy);
        assert_eq!(post.get_phase(), Phase::PostDeploy);
    }
}
