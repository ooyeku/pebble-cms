use crate::cli::MigrateCommand;
use crate::{Config, Database};
use anyhow::Result;
use std::io::{self, Write};
use std::path::Path;

pub async fn run(config_path: &Path, command: Option<MigrateCommand>) -> Result<()> {
    let config = Config::load(config_path)?;
    let db = Database::open(&config.database.path)?;

    match command {
        None => {
            // Default: run forward migrations (backward compatible)
            db.migrate()?;
            tracing::info!("Migrations complete");
        }
        Some(MigrateCommand::Status) => {
            show_status(&db)?;
        }
        Some(MigrateCommand::Rollback { steps, force }) => {
            rollback(&db, steps, force)?;
        }
    }

    Ok(())
}

fn show_status(db: &Database) -> Result<()> {
    let statuses = db.get_migration_status()?;

    println!("\n  Migration Status\n");
    println!("  {:<10} {:<45} {}", "Version", "Description", "Applied");
    println!("  {}", "-".repeat(80));

    let descriptions = [
        "Core tables (users, content, tags, media, settings)",
        "Full-text search (FTS5)",
        "Media optimization columns",
        "Scheduled publishing",
        "Analytics tables",
        "Content versioning",
        "Audit logging",
        "Preview tokens",
        "Content series",
        "API tokens and webhooks",
    ];

    for (version, applied_at) in &statuses {
        let desc = descriptions
            .get((*version as usize).saturating_sub(1))
            .unwrap_or(&"Unknown migration");

        let applied = match applied_at {
            Some(ts) => format!("\x1b[32m✓\x1b[0m {}", ts),
            None => "\x1b[33m✗ pending\x1b[0m".to_string(),
        };

        println!(
            "  {:<10} {:<45} {}",
            format!("{:03}", version),
            desc,
            applied
        );
    }

    let applied_count = statuses.iter().filter(|(_, ts)| ts.is_some()).count();
    let pending_count = statuses.len() - applied_count;

    println!();
    if pending_count > 0 {
        println!(
            "  {} applied, {} pending. Run `pebble migrate` to apply.",
            applied_count, pending_count
        );
    } else {
        println!("  All {} migrations applied.", applied_count);
    }
    println!();

    Ok(())
}

fn rollback(db: &Database, steps: u32, force: bool) -> Result<()> {
    let statuses = db.get_migration_status()?;
    let applied: Vec<i32> = statuses
        .iter()
        .filter(|(_, ts)| ts.is_some())
        .map(|(v, _)| *v)
        .collect();

    if applied.is_empty() {
        println!("No migrations to roll back.");
        return Ok(());
    }

    let to_rollback: Vec<i32> = applied.iter().rev().take(steps as usize).copied().collect();

    if to_rollback.is_empty() {
        println!("Nothing to roll back.");
        return Ok(());
    }

    // Safety check: rolling back migration 001 requires --force
    if to_rollback.contains(&1) && !force {
        anyhow::bail!(
            "Rolling back migration 001 will DESTROY ALL DATA (users, content, tags, media).\n\
             This cannot be undone. Use --force to confirm."
        );
    }

    // Show what will be rolled back
    let descriptions = [
        "Core tables (users, content, tags, media, settings)",
        "Full-text search (FTS5)",
        "Media optimization columns",
        "Scheduled publishing",
        "Analytics tables",
        "Content versioning",
        "Audit logging",
        "Preview tokens",
        "Content series",
        "API tokens and webhooks",
    ];

    println!("\n  The following migrations will be rolled back:\n");
    for v in &to_rollback {
        let desc = descriptions
            .get((*v as usize).saturating_sub(1))
            .unwrap_or(&"Unknown");
        println!("    {:03} — {}", v, desc);
    }

    // Prompt for confirmation unless --force
    if !force {
        print!("\n  This will delete data. Continue? [y/N] ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("  Rollback cancelled.");
            return Ok(());
        }
    }

    // Execute rollbacks in reverse order (highest version first)
    for version in &to_rollback {
        println!("  Rolling back migration {:03}...", version);
        db.rollback_migration(*version)?;
    }

    println!(
        "\n  Successfully rolled back {} migration(s).",
        to_rollback.len()
    );
    println!("  Run `pebble migrate` to re-apply them.\n");

    Ok(())
}
