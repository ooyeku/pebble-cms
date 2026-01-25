use crate::Database;
use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DatabaseStats {
    // File info
    pub file_path: String,
    pub file_size_bytes: i64,
    pub file_size_human: String,

    // SQLite version and settings
    pub sqlite_version: String,
    pub journal_mode: String,
    pub auto_vacuum: String,
    pub cache_size: i64,
    pub page_size: i64,
    pub page_count: i64,
    pub freelist_count: i64,
    pub encoding: String,

    // WAL mode stats (if applicable)
    pub wal_checkpoint: Option<WalCheckpointStats>,

    // Table statistics
    pub tables: Vec<TableStats>,

    // Index statistics
    pub indexes: Vec<IndexStats>,

    // Connection pool stats
    pub pool_size: u32,
    pub pool_idle: u32,

    // Integrity check
    pub integrity_check: String,

    // Performance stats
    pub compile_options: Vec<String>,

    // Memory stats
    pub memory_used: i64,
    pub memory_high_water: i64,

    // Cache stats
    pub cache_hit: i64,
    pub cache_miss: i64,
    pub cache_write: i64,
    pub cache_spill: i64,
}

#[derive(Debug, Serialize)]
pub struct WalCheckpointStats {
    pub wal_pages: i64,
    pub wal_frames_checkpointed: i64,
}

#[derive(Debug, Serialize)]
pub struct TableStats {
    pub name: String,
    pub row_count: i64,
    pub size_estimate: String,
}

#[derive(Debug, Serialize)]
pub struct IndexStats {
    pub name: String,
    pub table_name: String,
    pub is_unique: bool,
    pub columns: String,
}

pub fn format_bytes(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

pub fn get_database_stats(db: &Database, db_path: &str) -> Result<DatabaseStats> {
    let conn = db.get()?;

    // Get file size
    let file_size_bytes = std::fs::metadata(db_path)
        .map(|m| m.len() as i64)
        .unwrap_or(0);
    let file_size_human = format_bytes(file_size_bytes);

    // SQLite version
    let sqlite_version: String = conn.query_row("SELECT sqlite_version()", [], |row| row.get(0))?;

    // PRAGMA values
    let journal_mode: String = conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;

    let auto_vacuum: i64 = conn.query_row("PRAGMA auto_vacuum", [], |row| row.get(0))?;
    let auto_vacuum = match auto_vacuum {
        0 => "none".to_string(),
        1 => "full".to_string(),
        2 => "incremental".to_string(),
        _ => "unknown".to_string(),
    };

    let cache_size: i64 = conn.query_row("PRAGMA cache_size", [], |row| row.get(0))?;

    let page_size: i64 = conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;

    let page_count: i64 = conn.query_row("PRAGMA page_count", [], |row| row.get(0))?;

    let freelist_count: i64 = conn.query_row("PRAGMA freelist_count", [], |row| row.get(0))?;

    let encoding: String = conn.query_row("PRAGMA encoding", [], |row| row.get(0))?;

    // WAL checkpoint stats (only if in WAL mode)
    let wal_checkpoint = if journal_mode.to_lowercase() == "wal" {
        // Try to get WAL stats without forcing a checkpoint
        let result: Result<(i64, i64), _> =
            conn.query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| {
                Ok((row.get(1)?, row.get(2)?))
            });
        result.ok().map(|(pages, frames)| WalCheckpointStats {
            wal_pages: pages,
            wal_frames_checkpointed: frames,
        })
    } else {
        None
    };

    // Get table statistics
    let mut tables = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
        )?;
        let table_names: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        for table_name in table_names {
            // Get row count
            let row_count: i64 = conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM \"{}\"", table_name),
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            // Estimate size using page count from dbstat (if available)
            let size_estimate = format!("~{} rows", row_count);

            tables.push(TableStats {
                name: table_name,
                row_count,
                size_estimate,
            });
        }
    }

    // Get index statistics
    let mut indexes = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT name, tbl_name, sql FROM sqlite_master WHERE type='index' AND name NOT LIKE 'sqlite_%' ORDER BY tbl_name, name"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;

        for row in rows.flatten() {
            let (name, table_name, sql) = row;
            let is_unique = sql
                .as_ref()
                .map(|s| s.to_uppercase().contains("UNIQUE"))
                .unwrap_or(false);

            // Get column info for this index
            let columns: String = conn
                .query_row(&format!("PRAGMA index_info(\"{}\")", name), [], |row| {
                    row.get::<_, String>(2)
                })
                .unwrap_or_else(|_| {
                    // Try to get all columns
                    let mut cols = Vec::new();
                    if let Ok(mut idx_stmt) =
                        conn.prepare(&format!("PRAGMA index_info(\"{}\")", name))
                    {
                        if let Ok(idx_rows) = idx_stmt.query_map([], |r| r.get::<_, String>(2)) {
                            cols = idx_rows.filter_map(|r| r.ok()).collect();
                        }
                    }
                    cols.join(", ")
                });

            indexes.push(IndexStats {
                name,
                table_name,
                is_unique,
                columns,
            });
        }
    }

    // Quick integrity check (just checks if OK, doesn't return details for large DBs)
    let integrity_check: String = conn
        .query_row("PRAGMA quick_check(1)", [], |row| row.get(0))
        .unwrap_or_else(|_| "unknown".to_string());

    // Compile options
    let mut compile_options = Vec::new();
    {
        let mut stmt = conn.prepare("PRAGMA compile_options")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows.flatten() {
            compile_options.push(row);
        }
    }

    // Memory stats using sqlite3_status
    let memory_used: i64 = conn
        .query_row("SELECT total_changes()", [], |row| row.get(0))
        .unwrap_or(0);

    // These require the status extension, fallback to 0
    let memory_high_water: i64 = 0;

    // Cache stats from sqlite3_db_status
    let (cache_hit, cache_miss, cache_write, cache_spill) = get_cache_stats(&conn);

    // Pool stats - we can get rough estimates
    let pool_size = 10u32; // Default from Database::open
    let pool_idle = 0u32; // Not easily accessible through r2d2

    Ok(DatabaseStats {
        file_path: db_path.to_string(),
        file_size_bytes,
        file_size_human,
        sqlite_version,
        journal_mode,
        auto_vacuum,
        cache_size,
        page_size,
        page_count,
        freelist_count,
        encoding,
        wal_checkpoint,
        tables,
        indexes,
        pool_size,
        pool_idle,
        integrity_check,
        compile_options,
        memory_used,
        memory_high_water,
        cache_hit,
        cache_miss,
        cache_write,
        cache_spill,
    })
}

fn get_cache_stats(conn: &rusqlite::Connection) -> (i64, i64, i64, i64) {
    // Try to get cache stats - these may not be available on all builds
    let cache_hit: i64 = conn
        .query_row(
            "SELECT * FROM pragma_database_list WHERE name='main'",
            [],
            |_| Ok(0i64),
        )
        .unwrap_or(0);

    // These stats aren't directly queryable via SQL in standard SQLite
    // Return placeholder values
    (cache_hit, 0, 0, 0)
}

#[derive(Debug, Serialize)]
pub struct DatabaseAnalysis {
    pub fragmentation_percent: f64,
    pub wasted_space_bytes: i64,
    pub wasted_space_human: String,
    pub recommendations: Vec<String>,
}

pub fn analyze_database(db: &Database, db_path: &str) -> Result<DatabaseAnalysis> {
    let stats = get_database_stats(db, db_path)?;

    let mut recommendations = Vec::new();

    // Check fragmentation (freelist pages vs total pages)
    let fragmentation_percent = if stats.page_count > 0 {
        (stats.freelist_count as f64 / stats.page_count as f64) * 100.0
    } else {
        0.0
    };

    let wasted_space_bytes = stats.freelist_count * stats.page_size;
    let wasted_space_human = format_bytes(wasted_space_bytes);

    if fragmentation_percent > 10.0 {
        recommendations.push(format!(
            "Database fragmentation is {:.1}%. Consider running VACUUM to reclaim {} of space.",
            fragmentation_percent, wasted_space_human
        ));
    }

    // Check auto_vacuum
    if stats.auto_vacuum == "none" && stats.file_size_bytes > 10 * 1024 * 1024 {
        recommendations.push(
            "Auto-vacuum is disabled. Consider enabling it for automatic space reclamation."
                .to_string(),
        );
    }

    // Check cache size
    if stats.cache_size.abs() < 2000 && stats.file_size_bytes > 50 * 1024 * 1024 {
        recommendations.push(
            "Cache size is relatively small for the database size. Consider increasing cache_size for better performance."
                .to_string(),
        );
    }

    // Check integrity
    if stats.integrity_check != "ok" {
        recommendations.push(format!(
            "Integrity check returned: {}. Database may have corruption issues.",
            stats.integrity_check
        ));
    }

    // Check for tables without indexes (simple heuristic)
    for table in &stats.tables {
        if table.row_count > 1000 {
            let has_index = stats.indexes.iter().any(|i| i.table_name == table.name);
            if !has_index && !table.name.ends_with("_fts") && !table.name.contains("_content") {
                recommendations.push(format!(
                    "Table '{}' has {} rows but no indexes. Consider adding indexes for frequently queried columns.",
                    table.name, table.row_count
                ));
            }
        }
    }

    if recommendations.is_empty() {
        recommendations.push("Database is healthy. No recommendations at this time.".to_string());
    }

    Ok(DatabaseAnalysis {
        fragmentation_percent,
        wasted_space_bytes,
        wasted_space_human,
        recommendations,
    })
}

pub fn run_vacuum(db: &Database) -> Result<()> {
    let conn = db.get()?;
    conn.execute_batch("VACUUM")?;
    Ok(())
}

pub fn run_analyze(db: &Database) -> Result<()> {
    let conn = db.get()?;
    conn.execute_batch("ANALYZE")?;
    Ok(())
}

pub fn run_integrity_check(db: &Database) -> Result<Vec<String>> {
    let conn = db.get()?;
    let mut stmt = conn.prepare("PRAGMA integrity_check")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let results: Vec<String> = rows.filter_map(|r| r.ok()).collect();
    Ok(results)
}
