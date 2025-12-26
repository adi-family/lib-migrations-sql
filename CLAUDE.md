lib-migrations-sql, rust, migrations, sql, sqlite

## Overview
- SQL migration layer built on lib-migrations-core
- SqlMigration for SQL-string-based migrations
- SqliteStore for SQLite migration tracking
- SqliteContext for SQLite execution

## Features
- `sqlite` (default) - SQLite support via rusqlite

## Usage
```rust
use lib_migrations_sql::{SqlMigration, SqliteStore, SqliteContext, MigrationRunner};

let store = SqliteStore::open("migrations.db")?;
let mut ctx = SqliteContext::open("app.db")?;

let mut runner = MigrationRunner::new(store)
    .add(SqlMigration::new(1, "create_users",
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
        .with_down("DROP TABLE users"))
    .add(SqlMigration::new(2, "add_email",
        "ALTER TABLE users ADD COLUMN email TEXT"));

runner.init()?;
runner.migrate(&mut ctx)?;
```

## Custom Database
Implement `SqlExecutor` for your database:
```rust
impl SqlExecutor for MyDatabase {
    type Error = MyError;
    fn execute(&mut self, sql: &str) -> Result<(), Self::Error> {
        self.run_sql(sql)
    }
}
```

## Separate Store and Context
- Store tracks which migrations applied (_migrations table)
- Context executes the actual SQL
- Can be same or different databases
