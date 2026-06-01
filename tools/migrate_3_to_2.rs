//! 一次性迁移工具：把 instance id 从 `{template_id}-{model_id}-{alias}`
//! 改为 `{template_id}-{alias}`。
//!
//! 使用方法：
//!     cargo run --release --bin migrate_3_to_2 [--dry-run] [--db-path <PATH>]
//!
//! 默认 DB 路径：~/.cc-switch-tui/db.sqlite
//!
//! 注：本工具是 v0.2.x → v0.3.x 升级用的一次性脚本，跑完可删。
//! 如果 db_path 不存在、或者所有 instance id 已是 2 段，工具会安全 noop。
//! 每次执行前自动备份原 DB。

use rusqlite::Connection;
use std::env;
use std::path::PathBuf;

struct Config {
    db_path: PathBuf,
    dry_run: bool,
}

impl Config {
    fn from_args() -> Result<Self, Box<dyn std::error::Error>> {
        let mut db_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cc-switch-tui")
            .join("db.sqlite");
        let mut dry_run = false;
        let args: Vec<String> = env::args().collect();
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--dry-run" => dry_run = true,
                "--db-path" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--db-path 需要一个路径参数".into());
                    }
                    db_path = PathBuf::from(&args[i]);
                }
                "--help" | "-h" => {
                    println!("一次性迁移：instance id 3 段 → 2 段");
                    println!();
                    println!("用法：cargo run --release --bin migrate_3_to_2 [OPTIONS]");
                    println!();
                    println!("选项：");
                    println!("    --dry-run          打印将迁移的内容但不改 DB");
                    println!("    --db-path <PATH>   DB 路径 (默认 ~/.cc-switch-tui/db.sqlite)");
                    std::process::exit(0);
                }
                _ => return Err(format!("未知参数: {}", args[i]).into()),
            }
            i += 1;
        }
        Ok(Config { db_path, dry_run })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_args()?;
    println!("DB 路径: {}", config.db_path.display());

    if !config.db_path.exists() {
        // DB 不存在 = 用户是全新安装，无需迁移
        println!("DB 不存在，无需迁移（全新安装）");
        return Ok(());
    }

    // 备份
    if !config.dry_run {
        let ts = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let backup_path = config
            .db_path
            .with_extension(format!("sqlite.backup.3to2.{}", ts));
        std::fs::copy(&config.db_path, &backup_path)?;
        println!("已备份: {}", backup_path.display());
    }

    let conn = Connection::open(&config.db_path)?;

    // 找出所有 instance：检测 id 是否是旧 3 段格式
    let mut stmt = conn.prepare(
        "SELECT id, template_id, model_id, alias FROM instances WHERE alias != ''",
    )?;
    let all: Vec<(String, String, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    // 旧 id = `{template_id}-{model_id}-{alias}`，新 id = `{template_id}-{alias}`
    let to_migrate: Vec<(String, String, String)> = all
        .iter()
        .filter_map(|(id, template_id, model_id, alias)| {
            let prefix = format!("{}-{}-", template_id, model_id);
            if id.starts_with(&prefix) {
                let new_id = format!("{}-{}", template_id, alias);
                Some((id.clone(), new_id, alias.clone()))
            } else {
                None
            }
        })
        .collect();

    if to_migrate.is_empty() {
        println!("无需迁移（所有 instance id 已是 2 段）");
        return Ok(());
    }

    println!("将迁移 {} 个 instance:", to_migrate.len());
    for (old, new, alias) in &to_migrate {
        println!("  {} -> {} (alias: {})", old, new, alias);
    }

    if config.dry_run {
        println!();
        println!("DRY RUN - 未修改");
        return Ok(());
    }

    // 冲突检测：新 id 不能和已存在的 instance 冲突
    for (old_id, new_id, _) in &to_migrate {
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM instances WHERE id = ?1 AND id != ?2",
                [new_id, old_id],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if exists {
            return Err(format!(
                "冲突：新 id '{}' 已存在（迁移 {} 会覆盖）",
                new_id, old_id
            )
            .into());
        }
    }

    // 执行迁移
    println!();
    for (old_id, new_id, _) in &to_migrate {
        let changes = conn.execute(
            "UPDATE instances SET id = ?1 WHERE id = ?2",
            [new_id, old_id],
        )?;
        println!("  {} -> {} (rows: {})", old_id, new_id, changes);
    }

    println!();
    println!("迁移完成");
    Ok(())
}
