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
        Self { pool: self.pool.clone() }
    }
}

impl Database {
    pub fn open(path: &str) -> Result<Self> {
        let path = Path::new(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::builder().max_size(10).build(manager)?;

        let conn = pool.get()?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

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
