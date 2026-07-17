use crate::dao::Dao;
use crate::domain::instance::validate_alias;
use crate::domain::{AppError, ProviderInstance, ProviderTemplate};
use rusqlite::Connection;
use std::path::Path;

pub struct SqliteDaoImpl {
    conn: Connection,
    templates: Vec<ProviderTemplate>,
    instances: Vec<ProviderInstance>,
}

impl SqliteDaoImpl {
    fn db<T>(result: Result<T, rusqlite::Error>) -> Result<T, AppError> {
        result.map_err(|e| AppError::Database(e.to_string()))
    }

    pub fn new(path: &str, templates: Vec<ProviderTemplate>) -> Result<Self, AppError> {
        if path != ":memory:"
            && let Some(parent) = Path::new(path).parent()
        {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::Database(format!("创建目录失败: {}", e)))?;
        }
        let conn = Self::db(Connection::open(path))?;
        Self::db(conn.execute(
            "CREATE TABLE IF NOT EXISTS instances (
                id TEXT PRIMARY KEY,
                template_id TEXT NOT NULL,
                model_id TEXT NOT NULL,
                api_key TEXT NOT NULL,
                created_at TEXT NOT NULL,
                alias TEXT NOT NULL DEFAULT '',
                opencode_model_id TEXT NOT NULL DEFAULT ''
            )",
            [],
        ))?;
        // 兼容旧表：添加缺失的列（PRAGMA table_info 查询列是否存在）
        let columns: Vec<String> =
            Self::db(conn.prepare("SELECT name FROM pragma_table_info('instances')"))?
                .query_map([], |row| row.get(0))
                .map_err(|e| AppError::Database(e.to_string()))?
                .collect::<Result<_, _>>()
                .map_err(|e| AppError::Database(e.to_string()))?;
        if !columns.contains(&"alias".to_string()) {
            let _ = conn.execute(
                "ALTER TABLE instances ADD COLUMN alias TEXT NOT NULL DEFAULT ''",
                [],
            );
        }
        if !columns.contains(&"opencode_model_id".to_string()) {
            let _ = conn.execute(
                "ALTER TABLE instances ADD COLUMN opencode_model_id TEXT NOT NULL DEFAULT ''",
                [],
            );
        }
        if !columns.contains(&"kv_cache_enabled".to_string()) {
            let _ = conn.execute(
                "ALTER TABLE instances ADD COLUMN kv_cache_enabled INTEGER NOT NULL DEFAULT 0",
                [],
            );
        }
        // 一次性迁移（幂等）：旧 MiniMax-M3 model id 永久 rename 到新 id
        let _ = conn.execute(
            "UPDATE instances SET model_id = 'MiniMax-M3[1m]' WHERE model_id = 'MiniMax-M3'",
            [],
        );
        // DROP COLUMN context_window_enabled（SQLite ≥ 3.35 支持；列不存在时失败被忽略）
        if columns.contains(&"context_window_enabled".to_string()) {
            let _ = conn.execute(
                "ALTER TABLE instances DROP COLUMN context_window_enabled",
                [],
            );
        }
        let mut dao = Self {
            conn,
            templates,
            instances: Vec::new(),
        };
        dao.refresh_instances()?;
        Ok(dao)
    }

    fn refresh_instances(&mut self) -> Result<(), AppError> {
        let mut stmt = Self::db(self.conn.prepare(
            "SELECT id, template_id, model_id, api_key, created_at, alias, opencode_model_id, kv_cache_enabled FROM instances"
        ))?;
        let rows = Self::db(stmt.query_map([], |row| {
            Ok(ProviderInstance {
                id: row.get(0)?,
                template_id: row.get(1)?,
                model_id: row.get(2)?,
                api_key: row.get(3)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?
                    .with_timezone(&chrono::Utc),
                alias: row.get(5)?,
                opencode_model_id: row.get(6)?,
                kv_cache_enabled: row.get::<_, i32>("kv_cache_enabled")? != 0,
            })
        }))?;
        self.instances.clear();
        for row in rows {
            self.instances.push(Self::db(row)?);
        }
        Ok(())
    }
}

impl Dao for SqliteDaoImpl {
    fn get_templates(&self) -> Vec<&ProviderTemplate> {
        self.templates.iter().collect()
    }

    fn get_template(&self, id: &str) -> Option<&ProviderTemplate> {
        self.templates.iter().find(|t| t.id == id)
    }

    fn list_instances(&self) -> Vec<&ProviderInstance> {
        self.instances.iter().collect()
    }

    fn get_instance(&self, id: &str) -> Option<&ProviderInstance> {
        self.instances.iter().find(|i| i.id == id)
    }

    fn create_instance(&mut self, instance: ProviderInstance) -> Result<(), AppError> {
        validate_alias(&instance.alias)?;
        let created_at_str = instance.created_at.to_rfc3339();
        match self.conn.execute(
            "INSERT INTO instances (id, template_id, model_id, api_key, created_at, alias, opencode_model_id, kv_cache_enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &instance.id,
                &instance.template_id,
                &instance.model_id,
                &instance.api_key,
                created_at_str,
                &instance.alias,
                &instance.opencode_model_id,
                instance.kv_cache_enabled as i32,
            ],
        ) {
            Ok(_) => {
                self.refresh_instances()?;
                Ok(())
            }
            Err(rusqlite::Error::SqliteFailure(ref err, _))
                if err.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY =>
            {
                Err(AppError::InstanceAlreadyExists(instance.id.clone()))
            }
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    fn delete_instance(&mut self, id: &str) -> Result<(), AppError> {
        let changes = Self::db(
            self.conn
                .execute("DELETE FROM instances WHERE id = ?1", [id]),
        )?;
        if changes == 0 {
            return Err(AppError::InstanceNotFound(id.to_string()));
        }
        self.refresh_instances()?;
        Ok(())
    }

    fn update_instance(
        &mut self,
        id: &str,
        model_id: String,
        alias: String,
        api_key: String,
    ) -> Result<(), AppError> {
        validate_alias(&alias)?;
        let changes = Self::db(self.conn.execute(
            "UPDATE instances SET model_id = ?1, alias = ?2, api_key = ?3 WHERE id = ?4",
            rusqlite::params![model_id, alias, api_key, id.to_string()],
        ))?;
        if changes == 0 {
            return Err(AppError::InstanceNotFound(id.to_string()));
        }
        self.refresh_instances()?;
        Ok(())
    }

    fn set_alias(&mut self, id: &str, alias: String) -> Result<(), AppError> {
        validate_alias(&alias)?;
        let changes = Self::db(self.conn.execute(
            "UPDATE instances SET alias = ?1 WHERE id = ?2",
            [alias, id.to_string()],
        ))?;
        if changes == 0 {
            return Err(AppError::InstanceNotFound(id.to_string()));
        }
        self.refresh_instances()?;
        Ok(())
    }

    fn set_opencode_model_id(
        &mut self,
        id: &str,
        opencode_model_id: String,
    ) -> Result<(), AppError> {
        let changes = Self::db(self.conn.execute(
            "UPDATE instances SET opencode_model_id = ?1 WHERE id = ?2",
            [opencode_model_id, id.to_string()],
        ))?;
        if changes == 0 {
            return Err(AppError::InstanceNotFound(id.to_string()));
        }
        self.refresh_instances()?;
        Ok(())
    }

    fn set_kv_cache_enabled(&mut self, id: &str, enabled: bool) -> Result<(), AppError> {
        let changes = Self::db(self.conn.execute(
            "UPDATE instances SET kv_cache_enabled = ?1 WHERE id = ?2",
            rusqlite::params![enabled as i32, id],
        ))?;
        if changes == 0 {
            return Err(AppError::InstanceNotFound(id.to_string()));
        }
        self.refresh_instances()?;
        Ok(())
    }

    fn rename_instance(
        &mut self,
        old_id: &str,
        new_id: &str,
        alias: String,
    ) -> Result<(), AppError> {
        validate_alias(&alias)?;
        // Check if old_id exists
        let old_instance = self
            .instances
            .iter()
            .find(|i| i.id == old_id)
            .ok_or_else(|| AppError::InstanceNotFound(old_id.to_string()))?;

        // Check if new_id already exists
        if self.instances.iter().any(|i| i.id == new_id) {
            return Err(AppError::InstanceAlreadyExists(new_id.to_string()));
        }

        // Delete old instance
        let changes = Self::db(
            self.conn
                .execute("DELETE FROM instances WHERE id = ?1", [old_id]),
        )?;
        if changes == 0 {
            return Err(AppError::InstanceNotFound(old_id.to_string()));
        }

        // Insert new instance
        let created_at_str = old_instance.created_at.to_rfc3339();
        Self::db(self.conn.execute(
            "INSERT INTO instances (id, template_id, model_id, api_key, created_at, alias, opencode_model_id, kv_cache_enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                new_id,
                old_instance.template_id,
                old_instance.model_id,
                old_instance.api_key,
                created_at_str,
                alias,
                old_instance.opencode_model_id,
                old_instance.kv_cache_enabled as i32,
            ],
        ))?;

        self.refresh_instances()?;
        tracing::info!("dao rename_instance: {} -> {}", old_id, new_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::register_templates;
    use tempfile::TempDir;

    fn create_test_dao() -> SqliteDaoImpl {
        let templates = register_templates();
        SqliteDaoImpl::new(":memory:", templates).unwrap()
    }

    #[test]
    fn test_constructor_creates_table() {
        let dao = create_test_dao();
        let instances = dao.list_instances();
        assert!(instances.is_empty());
    }

    #[test]
    fn test_get_instance_returns_inserted() {
        let mut dao = create_test_dao();
        let instance = ProviderInstance {
            id: "minimax-MiniMax-M2.7-highspeed".to_string(),
            template_id: "minimax".to_string(),
            model_id: "MiniMax-M2.7-highspeed".to_string(),
            api_key: "test-key".to_string(),
            created_at: chrono::Utc::now(),
            alias: "cl-mini".to_string(),
            opencode_model_id: String::new(),
            kv_cache_enabled: false,
        };
        dao.create_instance(instance.clone()).unwrap();
        let found = dao.get_instance(&instance.id).unwrap();
        assert_eq!(found.id, instance.id);
        assert_eq!(found.api_key, instance.api_key);
    }

    #[test]
    fn test_set_alias_updates_alias() {
        let mut dao = create_test_dao();
        let instance = ProviderInstance {
            id: "minimax-MiniMax-M2.7-highspeed".to_string(),
            template_id: "minimax".to_string(),
            model_id: "MiniMax-M2.7-highspeed".to_string(),
            api_key: "key".to_string(),
            created_at: chrono::Utc::now(),
            alias: "cl-mini".to_string(),
            opencode_model_id: String::new(),
            kv_cache_enabled: false,
        };
        dao.create_instance(instance).unwrap();
        dao.set_alias("minimax-MiniMax-M2.7-highspeed", "cl-mini".to_string())
            .unwrap();
        let found = dao.get_instance("minimax-MiniMax-M2.7-highspeed").unwrap();
        assert_eq!(found.alias, "cl-mini");
    }

    #[test]
    fn test_update_instance_changes_api_key() {
        let mut dao = create_test_dao();
        let instance = ProviderInstance {
            id: "minimax-MiniMax-M2.7-highspeed".to_string(),
            template_id: "minimax".to_string(),
            model_id: "MiniMax-M2.7-highspeed".to_string(),
            api_key: "old-key".to_string(),
            created_at: chrono::Utc::now(),
            alias: "cl-mini".to_string(),
            opencode_model_id: String::new(),
            kv_cache_enabled: false,
        };
        dao.create_instance(instance).unwrap();
        dao.update_instance(
            "minimax-MiniMax-M2.7-highspeed",
            "MiniMax-M2.7-highspeed".to_string(),
            "cl-mini".to_string(),
            "new-key".to_string(),
        )
        .unwrap();
        let found = dao.get_instance("minimax-MiniMax-M2.7-highspeed").unwrap();
        assert_eq!(found.api_key, "new-key");
    }

    #[test]
    fn test_update_instance_not_found() {
        let mut dao = create_test_dao();
        let result = dao.update_instance(
            "nonexistent",
            "m".to_string(),
            "a".to_string(),
            "key".to_string(),
        );
        assert!(matches!(result, Err(AppError::InstanceNotFound(_))));
    }

    /// 新行为：update_instance 同时改 model_id + alias + api_key，
    /// 改 model 不影响 id 主键稳定性。
    #[test]
    fn test_update_instance_changes_model_id_preserves_id() {
        let mut dao = create_test_dao();
        let instance = ProviderInstance {
            id: "minimax-cl-mini".to_string(),
            template_id: "minimax".to_string(),
            model_id: "MiniMax-M2.7-highspeed".to_string(),
            api_key: "old-key".to_string(),
            created_at: chrono::Utc::now(),
            alias: "cl-mini".to_string(),
            opencode_model_id: String::new(),
            kv_cache_enabled: false,
        };
        dao.create_instance(instance).unwrap();

        // 改 model（M2.7 → M3），alias 和 api_key 同时更新
        dao.update_instance(
            "minimax-cl-mini",
            "MiniMax-M3".to_string(),
            "cl-mini".to_string(),
            "new-key".to_string(),
        )
        .unwrap();

        // id 不变（id 格式只含 template+alias，model 改变不动 id）
        let found = dao.get_instance("minimax-cl-mini").expect("id 保持稳定");
        assert_eq!(found.id, "minimax-cl-mini");
        assert_eq!(found.model_id, "MiniMax-M3");
        assert_eq!(found.api_key, "new-key");
    }

    #[test]
    fn test_delete_instance_removes_it() {
        let mut dao = create_test_dao();
        let instance = ProviderInstance {
            id: "minimax-MiniMax-M2.7-highspeed".to_string(),
            template_id: "minimax".to_string(),
            model_id: "MiniMax-M2.7-highspeed".to_string(),
            api_key: "key".to_string(),
            created_at: chrono::Utc::now(),
            alias: "cl-mini".to_string(),
            opencode_model_id: String::new(),
            kv_cache_enabled: false,
        };
        dao.create_instance(instance).unwrap();
        dao.delete_instance("minimax-MiniMax-M2.7-highspeed")
            .unwrap();
        assert!(dao.get_instance("minimax-MiniMax-M2.7-highspeed").is_none());
    }

    #[test]
    fn test_delete_instance_not_found() {
        let mut dao = create_test_dao();
        let result = dao.delete_instance("nonexistent");
        assert!(matches!(result, Err(AppError::InstanceNotFound(_))));
    }

    #[test]
    fn test_create_instance_duplicate() {
        let mut dao = create_test_dao();
        let instance = ProviderInstance {
            id: "minimax-MiniMax-M2.7-highspeed".to_string(),
            template_id: "minimax".to_string(),
            model_id: "MiniMax-M2.7-highspeed".to_string(),
            api_key: "key".to_string(),
            created_at: chrono::Utc::now(),
            alias: "cl-mini".to_string(),
            opencode_model_id: String::new(),
            kv_cache_enabled: false,
        };
        dao.create_instance(instance.clone()).unwrap();
        let result = dao.create_instance(instance);
        assert!(matches!(result, Err(AppError::InstanceAlreadyExists(_))));
    }

    #[test]
    fn test_rename_instance_updates_id_and_alias() {
        let mut dao = create_test_dao();
        let instance = ProviderInstance {
            id: "minimax-MiniMax-M2.7-highspeed".to_string(),
            template_id: "minimax".to_string(),
            model_id: "MiniMax-M2.7-highspeed".to_string(),
            api_key: "key".to_string(),
            created_at: chrono::Utc::now(),
            alias: "cl-mini".to_string(),
            opencode_model_id: String::new(),
            kv_cache_enabled: false,
        };
        dao.create_instance(instance).unwrap();

        // Rename: update id and alias together
        dao.rename_instance(
            "minimax-MiniMax-M2.7-highspeed",
            "minimax-MiniMax-M2.7-highspeed-cl-mini",
            "cl-mini".to_string(),
        )
        .unwrap();

        // Old id should not exist
        assert!(dao.get_instance("minimax-MiniMax-M2.7-highspeed").is_none());
        // New id should exist with new alias
        let found = dao
            .get_instance("minimax-MiniMax-M2.7-highspeed-cl-mini")
            .unwrap();
        assert_eq!(found.alias, "cl-mini");
    }

    #[test]
    fn test_rename_instance_new_id_conflict() {
        let mut dao = create_test_dao();
        let instance1 = ProviderInstance {
            id: "minimax-MiniMax-M2.7-highspeed".to_string(),
            template_id: "minimax".to_string(),
            model_id: "MiniMax-M2.7-highspeed".to_string(),
            api_key: "key1".to_string(),
            created_at: chrono::Utc::now(),
            alias: "cl-mini".to_string(),
            opencode_model_id: String::new(),
            kv_cache_enabled: false,
        };
        let instance2 = ProviderInstance {
            id: "minimax-MiniMax-M2.7-highspeed-cl-mini".to_string(),
            template_id: "minimax".to_string(),
            model_id: "MiniMax-M2.7-highspeed".to_string(),
            api_key: "key2".to_string(),
            created_at: chrono::Utc::now(),
            alias: "cl-mini".to_string(),
            opencode_model_id: String::new(),
            kv_cache_enabled: false,
        };
        dao.create_instance(instance1).unwrap();
        dao.create_instance(instance2).unwrap();

        // Try to rename instance1 to instance2's id - should fail
        let result = dao.rename_instance(
            "minimax-MiniMax-M2.7-highspeed",
            "minimax-MiniMax-M2.7-highspeed-cl-mini",
            "cl-mini".to_string(),
        );
        assert!(matches!(result, Err(AppError::InstanceAlreadyExists(_))));
    }

    #[test]
    fn test_rename_instance_not_found() {
        let mut dao = create_test_dao();
        let result = dao.rename_instance("nonexistent", "some-new-id", "cl-new".to_string());
        assert!(matches!(result, Err(AppError::InstanceNotFound(_))));
    }

    #[test]
    fn test_migration_renames_old_minimax_m3_model_id() {
        // 模拟旧 DB：含 MiniMax-M3 行 + 含 context_window_enabled 列
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db").to_string_lossy().to_string();
        let conn = Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE instances (
                id TEXT PRIMARY KEY,
                template_id TEXT NOT NULL,
                model_id TEXT NOT NULL,
                api_key TEXT NOT NULL,
                created_at TEXT NOT NULL,
                alias TEXT NOT NULL DEFAULT '',
                opencode_model_id TEXT NOT NULL DEFAULT '',
                kv_cache_enabled INTEGER NOT NULL DEFAULT 0,
                context_window_enabled INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO instances (id, template_id, model_id, api_key, created_at, alias, opencode_model_id, kv_cache_enabled, context_window_enabled)
             VALUES ('test-1', 'minimax', 'MiniMax-M3', 'key', '2024-01-01T00:00:00Z', 'cl-test', '', 0, 1)",
            [],
        ).unwrap();
        drop(conn);

        // 启动新代码：旧 model_id 应被 rename 到 MiniMax-M3[1m]
        let templates = register_templates();
        let dao = SqliteDaoImpl::new(&db_path, templates).unwrap();
        let instance = dao.get_instance("test-1").unwrap();
        assert_eq!(
            instance.model_id, "MiniMax-M3[1m]",
            "旧 model_id='MiniMax-M3' 应被自动 rename 到 'MiniMax-M3[1m]'"
        );
    }

    #[test]
    fn test_migration_drops_context_window_enabled_column() {
        // 模拟旧 DB 含 context_window_enabled 列
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db").to_string_lossy().to_string();
        let conn = Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE instances (
                id TEXT PRIMARY KEY,
                template_id TEXT NOT NULL,
                model_id TEXT NOT NULL,
                api_key TEXT NOT NULL,
                created_at TEXT NOT NULL,
                alias TEXT NOT NULL DEFAULT '',
                opencode_model_id TEXT NOT NULL DEFAULT '',
                kv_cache_enabled INTEGER NOT NULL DEFAULT 0,
                context_window_enabled INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .unwrap();
        drop(conn);

        // 启动新代码：context_window_enabled 列应被 DROP
        let templates = register_templates();
        let _dao = SqliteDaoImpl::new(&db_path, templates).unwrap();

        let conn = Connection::open(&db_path).unwrap();
        let columns: Vec<String> = conn
            .prepare("SELECT name FROM pragma_table_info('instances')")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(
            !columns.contains(&"context_window_enabled".to_string()),
            "context_window_enabled 列应在启动时被 DROP；当前列: {columns:?}"
        );
    }

    #[test]
    fn test_migration_runs_once_on_old_db_and_is_idempotent() {
        // 旧 DB 状态：旧 model_id 行 + 旧 context_window_enabled 列同时存在
        // —— 这是生产环境升级路径的真实场景。
        // 验证：第一次启动同时触发 UPDATE rename + DROP COLUMN；第二次启动幂等无副作用。
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db").to_string_lossy().to_string();
        let conn = Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE instances (
                id TEXT PRIMARY KEY,
                template_id TEXT NOT NULL,
                model_id TEXT NOT NULL,
                api_key TEXT NOT NULL,
                created_at TEXT NOT NULL,
                alias TEXT NOT NULL DEFAULT '',
                opencode_model_id TEXT NOT NULL DEFAULT '',
                kv_cache_enabled INTEGER NOT NULL DEFAULT 0,
                context_window_enabled INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO instances (id, template_id, model_id, api_key, created_at, alias, opencode_model_id, kv_cache_enabled, context_window_enabled)
             VALUES ('test-1', 'minimax', 'MiniMax-M3', 'key', '2024-01-01T00:00:00Z', 'cl-test', '', 0, 1)",
            [],
        ).unwrap();
        drop(conn);

        // 第一次启动：rename + drop 都应执行
        let templates = register_templates();
        let dao1 = SqliteDaoImpl::new(&db_path, templates.clone()).unwrap();
        assert_eq!(
            dao1.get_instance("test-1").unwrap().model_id,
            "MiniMax-M3[1m]",
            "第一次启动后 model_id 应从 MiniMax-M3 rename 到 MiniMax-M3[1m]"
        );
        let columns_after_first: Vec<String> = Connection::open(&db_path)
            .unwrap()
            .prepare("SELECT name FROM pragma_table_info('instances')")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(
            !columns_after_first.contains(&"context_window_enabled".to_string()),
            "第一次启动后 context_window_enabled 列应被 DROP；当前列: {columns_after_first:?}"
        );

        // 第二次启动：幂等无副作用
        let dao2 = SqliteDaoImpl::new(&db_path, templates).unwrap();
        assert_eq!(dao2.list_instances().len(), 1, "第二次启动后行数应保持不变");
        assert_eq!(
            dao2.get_instance("test-1").unwrap().model_id,
            "MiniMax-M3[1m]",
            "第二次启动 model_id 应保持 MiniMax-M3[1m]"
        );
    }
}
