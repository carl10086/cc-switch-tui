use crate::dao::Dao;
use crate::domain::{AppError, ProviderInstance, ProviderTemplate};
use rusqlite::Connection;
use std::path::Path;

pub struct SqliteDaoImpl {
    conn: Connection,
    templates: Vec<ProviderTemplate>,
    instances: Vec<ProviderInstance>,
}

impl SqliteDaoImpl {
    pub fn new(path: &str, templates: Vec<ProviderTemplate>) -> Result<Self, rusqlite::Error> {
        if path != ":memory:" {
            if let Some(parent) = Path::new(path).parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    rusqlite::Error::InvalidPath(std::path::PathBuf::from(parent).join(e.to_string()))
                })?;
            }
        }
        let conn = Connection::open(path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS instances (
                id TEXT PRIMARY KEY,
                template_id TEXT NOT NULL,
                model_id TEXT NOT NULL,
                api_key TEXT NOT NULL,
                created_at TEXT NOT NULL,
                is_current INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;
        conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_current ON instances(is_current) WHERE is_current = 1",
            [],
        )?;
        let mut dao = Self { conn, templates, instances: Vec::new() };
        dao.refresh_instances()?;
        Ok(dao)
    }

    fn refresh_instances(&mut self) -> Result<(), rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, template_id, model_id, api_key, created_at FROM instances"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ProviderInstance {
                id: row.get(0)?,
                template_id: row.get(1)?,
                model_id: row.get(2)?,
                api_key: row.get(3)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    ))?
                    .with_timezone(&chrono::Utc),
            })
        })?;
        self.instances.clear();
        for row in rows {
            self.instances.push(row?);
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
        let created_at_str = instance.created_at.to_rfc3339();
        match self.conn.execute(
            "INSERT INTO instances (id, template_id, model_id, api_key, created_at, is_current)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            rusqlite::params![
                &instance.id,
                &instance.template_id,
                &instance.model_id,
                &instance.api_key,
                created_at_str,
            ],
        ) {
            Ok(_) => {
                self.refresh_instances().expect("Failed to refresh instances after create");
                Ok(())
            }
            Err(rusqlite::Error::SqliteFailure(ref err, _))
                if err.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY =>
            {
                Err(AppError::InstanceAlreadyExists(instance.id.clone()))
            }
            Err(e) => panic!("Database error in create_instance: {}", e),
        }
    }

    fn delete_instance(&mut self, _id: &str) -> Result<(), AppError> { Ok(()) }
    fn get_current_instance(&self) -> Option<&ProviderInstance> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM instances WHERE is_current = 1"
        ).ok()?;
        let id: String = stmt.query_row([], |row| row.get(0)).ok()?;
        self.instances.iter().find(|i| i.id == id)
    }
    fn set_current_instance(&mut self, _id: &str) -> Result<(), AppError> { Ok(()) }
    fn update_instance(&mut self, _id: &str, _api_key: String) -> Result<(), AppError> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::templates::register_templates;

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
        };
        dao.create_instance(instance.clone()).unwrap();
        let found = dao.get_instance(&instance.id).unwrap();
        assert_eq!(found.id, instance.id);
        assert_eq!(found.api_key, instance.api_key);
    }

    #[test]
    fn test_get_current_instance() {
        let mut dao = create_test_dao();
        let instance = ProviderInstance {
            id: "minimax-MiniMax-M2.7-highspeed".to_string(),
            template_id: "minimax".to_string(),
            model_id: "MiniMax-M2.7-highspeed".to_string(),
            api_key: "key".to_string(),
            created_at: chrono::Utc::now(),
        };
        dao.create_instance(instance).unwrap();
        dao.conn.execute(
            "UPDATE instances SET is_current = 1 WHERE id = ?1",
            ["minimax-MiniMax-M2.7-highspeed"],
        ).unwrap();
        dao.refresh_instances().unwrap();
        let current = dao.get_current_instance().unwrap();
        assert_eq!(current.id, "minimax-MiniMax-M2.7-highspeed");
    }
}
