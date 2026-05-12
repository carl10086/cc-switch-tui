//! Migration tool for instance ID format change
//!
//! Changes instance id from `{template_id}-{model_id}` to `{template_id}-{model_id}-{alias}`
//!
//! Usage:
//!     cargo run --release --bin migrate_instances_id [OPTIONS]
//!
//! Options:
//!     --dry-run          Print migration SQL without executing
//!     --no-backup        Skip backup before migration (enabled by default)
//!     --db-path <PATH>   Database path (default: ~/.cc-switch-tui/db.sqlite)

use rusqlite::Connection;
use std::env;
use std::path::PathBuf;
use chrono::Utc;

struct Config {
    db_path: PathBuf,
    dry_run: bool,
    backup: bool,
}

impl Config {
    fn from_args() -> Result<Self, Box<dyn std::error::Error>> {
        let mut db_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cc-switch-tui")
            .join("db.sqlite");
        let mut dry_run = false;
        let mut backup = true;

        let args: Vec<String> = env::args().collect();
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--dry-run" => {
                    dry_run = true;
                }
                "--no-backup" => {
                    backup = false;
                }
                "--db-path" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--db-path requires a path argument".into());
                    }
                    db_path = PathBuf::from(&args[i]);
                }
                "--help" | "-h" => {
                    println!("Migration tool for instance ID format change");
                    println!();
                    println!("Usage:");
                    println!("    cargo run --release --bin migrate_instances_id [OPTIONS]");
                    println!();
                    println!("Options:");
                    println!("    --dry-run          Print migration SQL without executing");
                    println!("    --no-backup        Skip backup before migration (enabled by default)");
                    println!("    --db-path <PATH>   Database path (default: ~/.cc-switch-tui/db.sqlite)");
                    println!("    --help, -h         Show this help message");
                    std::process::exit(0);
                }
                _ => {
                    return Err(format!("Unknown argument: {}", args[i]).into());
                }
            }
            i += 1;
        }

        Ok(Config {
            db_path,
            dry_run,
            backup,
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_args()?;

    println!("Database path: {}", config.db_path.display());

    if !config.db_path.exists() {
        return Err(format!("Database not found: {}", config.db_path.display()).into());
    }

    // Backup if not dry-run and backup is enabled
    if config.backup && !config.dry_run {
        let backup_path = config.db_path.with_extension(format!(
            "sqlite.backup.{}",
            Utc::now().format("%Y%m%d%H%M%S")
        ));
        println!("Creating backup: {}", backup_path.display());
        std::fs::copy(&config.db_path, &backup_path)?;
        println!("Backup created successfully");
    }

    let conn = Connection::open(&config.db_path)?;

    // Find all instances with non-empty alias and old-style id (no alias in id)
    let mut stmt = conn.prepare(
        "SELECT id, template_id, model_id, alias FROM instances WHERE alias != ''"
    )?;

    let instances: Vec<(String, String, String, String)> = stmt
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    println!("Found {} instances with non-empty alias", instances.len());

    // Check which ones need migration (id doesn't already contain alias)
    let mut to_migrate: Vec<(String, String, String, String)> = Vec::new();
    for (id, template_id, model_id, alias) in &instances {
        let expected_new_id = format!("{}-{}-{}", template_id, model_id, alias);
        if id != &expected_new_id {
            to_migrate.push((id.clone(), expected_new_id.clone(), template_id.clone(), alias.clone()));
        }
    }

    if to_migrate.is_empty() {
        println!("No instances need migration");
        return Ok(());
    }

    println!("Instances to migrate: {}", to_migrate.len());
    for (old_id, new_id, _, alias) in &to_migrate {
        println!("  {} -> {} (alias: {})", old_id, new_id, alias);
    }

    if config.dry_run {
        println!();
        println!("DRY RUN - no changes were made");
        return Ok(());
    }

    // Check for conflicts before migrating
    for (old_id, new_id, _, _) in &to_migrate {
        let exists: bool = conn.query_row(
            "SELECT 1 FROM instances WHERE id = ?1",
            [new_id],
            |_| Ok(true),
        ).unwrap_or(false);

        if exists {
            return Err(format!(
                "Conflict: new id '{}' already exists (migration would overwrite data from '{}')",
                new_id, old_id
            ).into());
        }
    }

    // Perform migration
    println!();
    println!("Performing migration...");
    for (old_id, new_id, _, _) in &to_migrate {
        let changes = conn.execute(
            "UPDATE instances SET id = ?1 WHERE id = ?2",
            [new_id, old_id],
        )?;
        println!("  Migrated: {} -> {} (rows affected: {})", old_id, new_id, changes);
    }

    println!();
    println!("Migration completed successfully!");

    Ok(())
}
