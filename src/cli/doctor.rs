use crate::services::database::{analyze_database, get_database_stats, run_integrity_check};
use crate::Config;
use crate::Database;
use anyhow::Result;
use std::path::Path;

#[derive(Debug)]
enum CheckStatus {
    Ok,
    Warn,
    Fail,
}

impl std::fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckStatus::Ok => write!(f, "\x1b[32m✓ OK\x1b[0m"),
            CheckStatus::Warn => write!(f, "\x1b[33m⚠ WARN\x1b[0m"),
            CheckStatus::Fail => write!(f, "\x1b[31m✗ FAIL\x1b[0m"),
        }
    }
}

struct CheckResult {
    name: String,
    status: CheckStatus,
    detail: String,
}

pub async fn run(config_path: &Path) -> Result<()> {
    println!("\n  Pebble Doctor — System Health Check\n");

    let mut results: Vec<CheckResult> = Vec::new();
    let mut has_failure = false;

    // 1. Config validity
    let config = match Config::load(config_path) {
        Ok(c) => {
            match c.validate() {
                Ok(()) => {
                    results.push(CheckResult {
                        name: "Configuration".into(),
                        status: CheckStatus::Ok,
                        detail: format!("Loaded from {}", config_path.display()),
                    });
                }
                Err(e) => {
                    results.push(CheckResult {
                        name: "Configuration".into(),
                        status: CheckStatus::Fail,
                        detail: format!("Validation error: {}", e),
                    });
                    has_failure = true;
                }
            }
            Some(c)
        }
        Err(e) => {
            results.push(CheckResult {
                name: "Configuration".into(),
                status: CheckStatus::Fail,
                detail: format!("Failed to load: {}", e),
            });
            has_failure = true;
            None
        }
    };

    // If config failed, we can't proceed with DB checks
    let config = match config {
        Some(c) => c,
        None => {
            print_results(&results);
            if has_failure {
                println!("\n  \x1b[31mSome checks failed. Fix the issues above before deploying.\x1b[0m\n");
            }
            return Ok(());
        }
    };

    // 2. Database connectivity
    let db = match Database::open(&config.database.path) {
        Ok(db) => {
            match db.health_check() {
                Ok(true) => {
                    results.push(CheckResult {
                        name: "Database connectivity".into(),
                        status: CheckStatus::Ok,
                        detail: format!("Connected to {}", config.database.path),
                    });
                }
                _ => {
                    results.push(CheckResult {
                        name: "Database connectivity".into(),
                        status: CheckStatus::Fail,
                        detail: "Health check returned unexpected result".into(),
                    });
                    has_failure = true;
                }
            }
            Some(db)
        }
        Err(e) => {
            results.push(CheckResult {
                name: "Database connectivity".into(),
                status: CheckStatus::Fail,
                detail: format!("Cannot open: {}", e),
            });
            has_failure = true;
            None
        }
    };

    let db = match db {
        Some(d) => d,
        None => {
            print_results(&results);
            if has_failure {
                println!("\n  \x1b[31mSome checks failed. Fix the issues above before deploying.\x1b[0m\n");
            }
            return Ok(());
        }
    };

    // 3. Database integrity
    match run_integrity_check(&db) {
        Ok(ref msgs) if msgs.len() == 1 && msgs[0] == "ok" => {
            results.push(CheckResult {
                name: "Database integrity".into(),
                status: CheckStatus::Ok,
                detail: "PRAGMA integrity_check passed".into(),
            });
        }
        Ok(msgs) => {
            let detail = msgs.join("; ");
            results.push(CheckResult {
                name: "Database integrity".into(),
                status: CheckStatus::Fail,
                detail: format!("Issues found: {}", detail),
            });
            has_failure = true;
        }
        Err(e) => {
            results.push(CheckResult {
                name: "Database integrity".into(),
                status: CheckStatus::Fail,
                detail: format!("Check failed: {}", e),
            });
            has_failure = true;
        }
    }

    // 4. Migration status
    {
        let conn = db.get()?;
        let current_version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let latest_version = 10;
        if current_version >= latest_version {
            results.push(CheckResult {
                name: "Migration status".into(),
                status: CheckStatus::Ok,
                detail: format!("All {} migrations applied", latest_version),
            });
        } else if current_version == 0 {
            results.push(CheckResult {
                name: "Migration status".into(),
                status: CheckStatus::Warn,
                detail: "No migrations applied. Run `pebble migrate`".into(),
            });
        } else {
            results.push(CheckResult {
                name: "Migration status".into(),
                status: CheckStatus::Warn,
                detail: format!(
                    "At version {}/{}. Run `pebble migrate` to apply pending migrations",
                    current_version, latest_version
                ),
            });
        }
    }

    // 5. Database fragmentation
    match analyze_database(&db, &config.database.path) {
        Ok(analysis) => {
            if analysis.fragmentation_percent > 10.0 {
                results.push(CheckResult {
                    name: "Database fragmentation".into(),
                    status: CheckStatus::Warn,
                    detail: format!(
                        "{:.1}% fragmented ({} wasted). Consider running VACUUM",
                        analysis.fragmentation_percent, analysis.wasted_space_human
                    ),
                });
            } else {
                results.push(CheckResult {
                    name: "Database fragmentation".into(),
                    status: CheckStatus::Ok,
                    detail: format!("{:.1}% fragmented", analysis.fragmentation_percent),
                });
            }
        }
        Err(e) => {
            results.push(CheckResult {
                name: "Database fragmentation".into(),
                status: CheckStatus::Warn,
                detail: format!("Could not analyze: {}", e),
            });
        }
    }

    // 6. Database file permissions
    {
        let db_path = Path::new(&config.database.path);
        match std::fs::metadata(db_path) {
            Ok(meta) => {
                if meta.permissions().readonly() {
                    results.push(CheckResult {
                        name: "Database permissions".into(),
                        status: CheckStatus::Warn,
                        detail: "Database file is read-only".into(),
                    });
                } else {
                    results.push(CheckResult {
                        name: "Database permissions".into(),
                        status: CheckStatus::Ok,
                        detail: "Writable".into(),
                    });
                }
            }
            Err(e) => {
                results.push(CheckResult {
                    name: "Database permissions".into(),
                    status: CheckStatus::Warn,
                    detail: format!("Cannot stat file: {}", e),
                });
            }
        }
    }

    // 7. Media directory
    {
        let media_dir = Path::new(&config.media.upload_dir);
        if media_dir.exists() {
            if media_dir.is_dir() {
                // Try to check writability by attempting to create a temp file
                let test_path = media_dir.join(".pebble_doctor_test");
                match std::fs::write(&test_path, b"test") {
                    Ok(()) => {
                        let _ = std::fs::remove_file(&test_path);
                        results.push(CheckResult {
                            name: "Media directory".into(),
                            status: CheckStatus::Ok,
                            detail: format!("{} (writable)", config.media.upload_dir),
                        });
                    }
                    Err(_) => {
                        results.push(CheckResult {
                            name: "Media directory".into(),
                            status: CheckStatus::Warn,
                            detail: format!("{} exists but is not writable", config.media.upload_dir),
                        });
                    }
                }
            } else {
                results.push(CheckResult {
                    name: "Media directory".into(),
                    status: CheckStatus::Warn,
                    detail: format!("{} exists but is not a directory", config.media.upload_dir),
                });
            }
        } else {
            results.push(CheckResult {
                name: "Media directory".into(),
                status: CheckStatus::Warn,
                detail: format!("{} does not exist. It will be created on first upload", config.media.upload_dir),
            });
        }
    }

    // 8. Disk space (Unix only)
    #[cfg(unix)]
    {
        let db_path = Path::new(&config.database.path);
        if let Some(parent) = db_path.parent() {
            if parent.exists() {
                // Use libc::statvfs for disk space checking
                match check_disk_space(parent) {
                    Some(available_mb) => {
                        if available_mb < 100 {
                            results.push(CheckResult {
                                name: "Disk space".into(),
                                status: CheckStatus::Warn,
                                detail: format!("Only {} MB available", available_mb),
                            });
                        } else {
                            results.push(CheckResult {
                                name: "Disk space".into(),
                                status: CheckStatus::Ok,
                                detail: format!("{} MB available", available_mb),
                            });
                        }
                    }
                    None => {
                        results.push(CheckResult {
                            name: "Disk space".into(),
                            status: CheckStatus::Warn,
                            detail: "Could not determine available disk space".into(),
                        });
                    }
                }
            }
        }
    }

    #[cfg(not(unix))]
    {
        results.push(CheckResult {
            name: "Disk space".into(),
            status: CheckStatus::Ok,
            detail: "Check skipped (non-Unix platform)".into(),
        });
    }

    // 9. Port availability
    {
        let test_addr = "127.0.0.1:8080";
        match std::net::TcpListener::bind(test_addr) {
            Ok(_listener) => {
                // Drop immediately — port is free
                results.push(CheckResult {
                    name: "Default port (8080)".into(),
                    status: CheckStatus::Ok,
                    detail: "Port 8080 is available".into(),
                });
            }
            Err(_) => {
                results.push(CheckResult {
                    name: "Default port (8080)".into(),
                    status: CheckStatus::Warn,
                    detail: "Port 8080 is in use. Use --port to specify an alternative".into(),
                });
            }
        }
    }

    // 10. Database stats (INFO only)
    match get_database_stats(&db, &config.database.path) {
        Ok(stats) => {
            let total_rows: i64 = stats.tables.iter().map(|t| t.row_count).sum();
            results.push(CheckResult {
                name: "Database stats".into(),
                status: CheckStatus::Ok,
                detail: format!(
                    "{}, {} tables, {} rows, SQLite {}",
                    stats.file_size_human,
                    stats.tables.len(),
                    total_rows,
                    stats.sqlite_version
                ),
            });
        }
        Err(e) => {
            results.push(CheckResult {
                name: "Database stats".into(),
                status: CheckStatus::Warn,
                detail: format!("Could not gather stats: {}", e),
            });
        }
    }

    print_results(&results);

    if has_failure {
        println!("\n  \x1b[31mSome checks failed. Fix the issues above before deploying.\x1b[0m\n");
    } else {
        println!("\n  \x1b[32mAll checks passed. Ready to deploy.\x1b[0m\n");
    }

    Ok(())
}

fn print_results(results: &[CheckResult]) {
    // Find the longest name for alignment
    let max_name_len = results.iter().map(|r| r.name.len()).max().unwrap_or(20);

    for (i, result) in results.iter().enumerate() {
        println!(
            "  {:>2}. {:<width$}  {}  {}",
            i + 1,
            result.name,
            result.status,
            result.detail,
            width = max_name_len,
        );
    }
}

#[cfg(unix)]
fn check_disk_space(path: &Path) -> Option<u64> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;

    let c_path = CString::new(path.to_str()?).ok()?;
    let mut stat = MaybeUninit::<libc::statvfs>::uninit();

    let result = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };

    if result == 0 {
        let stat = unsafe { stat.assume_init() };
        let available_bytes = stat.f_bavail as u64 * stat.f_frsize as u64;
        Some(available_bytes / (1024 * 1024))
    } else {
        None
    }
}
