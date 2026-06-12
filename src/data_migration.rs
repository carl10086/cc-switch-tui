//! 数据目录迁移：将旧版相对路径 `.cc-switch-tui/` 下的 SQLite 数据复制到 `~/.cc-switch-tui/`。

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// 迁移过程中可能发生的错误。
#[derive(Debug)]
pub enum DataMigrationError {
    Io(io::Error),
}

impl fmt::Display for DataMigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataMigrationError::Io(e) => write!(f, "data migration failed: {}", e),
        }
    }
}

impl From<io::Error> for DataMigrationError {
    fn from(e: io::Error) -> Self {
        DataMigrationError::Io(e)
    }
}

impl std::error::Error for DataMigrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DataMigrationError::Io(e) => Some(e),
        }
    }
}

impl From<DataMigrationError> for io::Error {
    fn from(e: DataMigrationError) -> Self {
        io::Error::new(io::ErrorKind::Other, e)
    }
}

/// 需要迁移的 SQLite 相关文件名。
const MIGRATED_SQLITE_FILES: &[&str] = &[
    "db.sqlite",
    "traces.sqlite",
    "traces.sqlite-wal",
    "traces.sqlite-shm",
];

/// 确保运行时数据位于 `home_cc_dir`。
///
/// 迁移规则：
/// 1. 若 `home_cc_dir/db.sqlite` 已存在，跳过（避免覆盖现有数据）。
/// 2. 若 `project_dir/.cc-switch-tui/db.sqlite` 不存在，跳过（无旧数据）。
/// 3. 否则将相关 SQLite 文件从项目目录复制到 home 目录，旧文件保留不动。
pub fn ensure_data_migrated(
    home_cc_dir: &Path,
    project_dir: &Path,
) -> Result<(), DataMigrationError> {
    let source_dir = project_dir.join(".cc-switch-tui");
    let source_db = source_dir.join("db.sqlite");
    let target_db = home_cc_dir.join("db.sqlite");

    // home 已有数据：以 home 为权威来源，不迁移。
    if target_db.exists() {
        return Ok(());
    }

    // 项目目录无旧数据：无需迁移。
    if !source_db.exists() {
        return Ok(());
    }

    fs::create_dir_all(home_cc_dir)?;

    for file in MIGRATED_SQLITE_FILES {
        let source = source_dir.join(file);
        let target = home_cc_dir.join(file);
        if source.exists() {
            fs::copy(&source, &target)?;
        }
    }

    tracing::info!(
        "migrated data from {} to {}",
        source_dir.display(),
        home_cc_dir.display()
    );

    Ok(())
}

/// 获取默认的数据目录 `~/.cc-switch-tui`。
/// 若无法获取 home 目录，则回退到当前工作目录下的 `.cc-switch-tui`。
pub fn default_cc_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cc-switch-tui")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_file(dir: &Path, name: &str, content: &[u8]) {
        fs::create_dir_all(dir).unwrap();
        let path = dir.join(name);
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content).unwrap();
    }

    fn read_file(dir: &Path, name: &str) -> Vec<u8> {
        fs::read(dir.join(name)).unwrap()
    }

    #[test]
    fn test_skip_when_home_db_exists() {
        let home = TempDir::new().unwrap();
        let project = TempDir::new().unwrap();

        let home_cc = home.path().join(".cc-switch-tui");
        let project_cc = project.path().join(".cc-switch-tui");

        // home 已有 db
        write_file(&home_cc, "db.sqlite", b"home-db");
        // project 也有 db
        write_file(&project_cc, "db.sqlite", b"project-db");

        ensure_data_migrated(&home_cc, project.path()).unwrap();

        // home 数据不应被覆盖
        assert_eq!(read_file(&home_cc, "db.sqlite"), b"home-db");
    }

    #[test]
    fn test_skip_when_no_project_data() {
        let home = TempDir::new().unwrap();
        let project = TempDir::new().unwrap();

        let home_cc = home.path().join(".cc-switch-tui");

        ensure_data_migrated(&home_cc, project.path()).unwrap();

        // home 不应被创建 db.sqlite
        assert!(!home_cc.join("db.sqlite").exists());
    }

    #[test]
    fn test_migrate_copies_sqlite_files() {
        let home = TempDir::new().unwrap();
        let project = TempDir::new().unwrap();

        let home_cc = home.path().join(".cc-switch-tui");
        let project_cc = project.path().join(".cc-switch-tui");

        write_file(&project_cc, "db.sqlite", b"project-db");
        write_file(&project_cc, "traces.sqlite", b"project-traces");

        ensure_data_migrated(&home_cc, project.path()).unwrap();

        assert_eq!(read_file(&home_cc, "db.sqlite"), b"project-db");
        assert_eq!(read_file(&home_cc, "traces.sqlite"), b"project-traces");
    }

    #[test]
    fn test_migrate_copies_wal_files() {
        let home = TempDir::new().unwrap();
        let project = TempDir::new().unwrap();

        let home_cc = home.path().join(".cc-switch-tui");
        let project_cc = project.path().join(".cc-switch-tui");

        write_file(&project_cc, "db.sqlite", b"project-db");
        write_file(&project_cc, "traces.sqlite", b"project-traces");
        write_file(&project_cc, "traces.sqlite-wal", b"wal");
        write_file(&project_cc, "traces.sqlite-shm", b"shm");

        ensure_data_migrated(&home_cc, project.path()).unwrap();

        assert_eq!(read_file(&home_cc, "traces.sqlite-wal"), b"wal");
        assert_eq!(read_file(&home_cc, "traces.sqlite-shm"), b"shm");
    }

    #[test]
    fn test_migrate_does_not_delete_source() {
        let home = TempDir::new().unwrap();
        let project = TempDir::new().unwrap();

        let home_cc = home.path().join(".cc-switch-tui");
        let project_cc = project.path().join(".cc-switch-tui");

        write_file(&project_cc, "db.sqlite", b"project-db");
        write_file(&project_cc, "traces.sqlite", b"project-traces");

        ensure_data_migrated(&home_cc, project.path()).unwrap();

        assert!(project_cc.join("db.sqlite").exists());
        assert!(project_cc.join("traces.sqlite").exists());
    }
}
