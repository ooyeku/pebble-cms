use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::path::Path;

pub type DbPool = Pool<SqliteConnectionManager>;

pub struct Database {
    pool: DbPool,
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}

impl Database {
    pub fn open(path: &str) -> Result<Self> {
        Self::open_with_pool_size(path, 10)
    }

    pub fn open_with_pool_size(path: &str, pool_size: u32) -> Result<Self> {
        let path = Path::new(path);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::builder().max_size(pool_size).build(manager)?;

        let conn = pool.get()?;
        // Production-safe SQLite tuning:
        // - WAL mode: concurrent reads during writes
        // - foreign_keys: enforce referential integrity
        // - busy_timeout: wait up to 5s instead of failing immediately on lock contention
        // - journal_size_limit: cap WAL file at 64MB to prevent unbounded growth
        // - synchronous=NORMAL: safe with WAL, much faster than FULL
        // - mmap_size: 128MB memory-mapped I/O for faster reads
        // - cache_size: ~64MB page cache (negative = KB)
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             PRAGMA busy_timeout=5000;
             PRAGMA journal_size_limit=67108864;
             PRAGMA synchronous=NORMAL;
             PRAGMA mmap_size=134217728;
             PRAGMA cache_size=-65536;",
        )?;

        Ok(Self { pool })
    }

    /// Check database connectivity and integrity.
    /// Returns Ok(true) if the database is healthy.
    pub fn health_check(&self) -> Result<bool> {
        let conn = self.get()?;
        let result: i32 = conn.query_row("SELECT 1", [], |row| row.get(0))?;
        Ok(result == 1)
    }

    pub fn open_memory(name: &str) -> Result<Self> {
        let uri = format!("file:{}?mode=memory&cache=shared", name);
        let manager = SqliteConnectionManager::file(&uri);
        let pool = Pool::builder()
            .max_size(5)
            .connection_timeout(std::time::Duration::from_secs(5))
            .build(manager)?;

        let conn = pool.get()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;

        Ok(Self { pool })
    }

    pub fn get(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>> {
        Ok(self.pool.get()?)
    }

    pub fn migrate(&self) -> Result<()> {
        let conn = self.get()?;
        run_migrations(&conn)?;
        Ok(())
    }

    /// Returns the status of all 10 migrations as (version, Option<applied_at>).
    /// Pending migrations have `None` for applied_at.
    pub fn get_migration_status(&self) -> Result<Vec<(i32, Option<String>)>> {
        let conn = self.get()?;

        // Ensure schema_migrations table exists
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT DEFAULT CURRENT_TIMESTAMP
            );",
        )?;

        let total_migrations = 10;
        let mut result = Vec::with_capacity(total_migrations);

        for version in 1..=total_migrations as i32 {
            let applied_at: Option<String> = conn
                .query_row(
                    "SELECT applied_at FROM schema_migrations WHERE version = ?1",
                    [version],
                    |row| row.get(0),
                )
                .ok();

            result.push((version, applied_at));
        }

        Ok(result)
    }

    /// Roll back a single migration by executing its rollback SQL
    /// and removing it from schema_migrations.
    pub fn rollback_migration(&self, version: i32) -> Result<()> {
        let conn = self.get()?;

        // Temporarily disable foreign keys for rollback
        conn.execute_batch("PRAGMA foreign_keys=OFF;")?;

        let rollback_sql = get_rollback_sql(version)?;
        conn.execute_batch(rollback_sql)?;
        conn.execute(
            "DELETE FROM schema_migrations WHERE version = ?1",
            [version],
        )?;

        // Re-enable foreign keys
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;

        Ok(())
    }
}

fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )?;

    let current_version: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let migrations: Vec<(i32, &str)> = vec![
        (1, include_str!("migrations/001_initial.sql")),
        (2, include_str!("migrations/002_fts.sql")),
        (3, include_str!("migrations/003_media_optimization.sql")),
        (4, include_str!("migrations/004_scheduled_publishing.sql")),
        (5, include_str!("migrations/005_analytics.sql")),
        (6, include_str!("migrations/006_content_versions.sql")),
        (7, include_str!("migrations/007_audit_log.sql")),
        (8, include_str!("migrations/008_preview_tokens.sql")),
        (9, include_str!("migrations/009_content_series.sql")),
        (10, include_str!("migrations/010_api_and_webhooks.sql")),
    ];

    for (version, sql) in migrations {
        if version > current_version {
            tracing::info!("Running migration {}", version);
            conn.execute_batch(sql)?;
            conn.execute(
                "INSERT INTO schema_migrations (version) VALUES (?)",
                [version],
            )?;
        }
    }

    Ok(())
}

fn get_rollback_sql(version: i32) -> Result<&'static str> {
    match version {
        1 => Ok(include_str!("migrations/001_rollback.sql")),
        2 => Ok(include_str!("migrations/002_rollback.sql")),
        3 => Ok(include_str!("migrations/003_rollback.sql")),
        4 => Ok(include_str!("migrations/004_rollback.sql")),
        5 => Ok(include_str!("migrations/005_rollback.sql")),
        6 => Ok(include_str!("migrations/006_rollback.sql")),
        7 => Ok(include_str!("migrations/007_rollback.sql")),
        8 => Ok(include_str!("migrations/008_rollback.sql")),
        9 => Ok(include_str!("migrations/009_rollback.sql")),
        10 => Ok(include_str!("migrations/010_rollback.sql")),
        _ => anyhow::bail!("No rollback SQL for migration version {}", version),
    }
}
