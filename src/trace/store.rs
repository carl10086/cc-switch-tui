//! SQLite-backed trace storage.

use std::path::Path;

use chrono::Utc;
use rusqlite::{Connection, Row};
use uuid::Uuid;

use super::models::{TraceDirection, TraceRecord, TraceSession};
use crate::domain::AppError;

/// Store for persisting trace sessions and records.
pub struct TraceStore {
    conn: Connection,
}

impl TraceStore {
    /// Open or create the trace database at `path`.
    pub fn new(path: &str) -> Result<Self, AppError> {
        if path != ":memory:" {
            if let Some(parent) = Path::new(path).parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    AppError::Database(format!("创建 trace 目录失败: {}", e))
                })?;
            }
        }
        let conn = Connection::open(path)
            .map_err(|e| AppError::Database(e.to_string()))?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| AppError::Database(e.to_string()))?;
        conn.pragma_update(None, "foreign_keys", true)
            .map_err(|e| AppError::Database(e.to_string()))?;
        Self::init_schema(&conn)?;
        Ok(Self { conn })
    }

    fn init_schema(conn: &Connection) -> Result<(), AppError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                started_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                date_key TEXT NOT NULL,
                alias TEXT NOT NULL,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                record_count INTEGER NOT NULL DEFAULT 0,
                summary_json TEXT
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS records (
                session_id TEXT NOT NULL,
                record_index INTEGER NOT NULL,
                turn INTEGER,
                timestamp TEXT,
                direction TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                claude_session_id TEXT,
                PRIMARY KEY (session_id, record_index),
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Migration: add claude_session_id to existing tables
        let _migrate: Result<_, _> = conn.execute(
            "ALTER TABLE records ADD COLUMN claude_session_id TEXT",
            [],
        );

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at)",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_date_key ON sessions(date_key)",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_records_session_id ON records(session_id)",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// Create a new trace session and return its id.
    pub fn create_session(
        &self,
        alias: &str,
        provider: &str,
        model: &str,
    ) -> Result<String, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let started_at = now.to_rfc3339();
        let date_key = now.date_naive().to_string();

        self.conn
            .execute(
                "INSERT INTO sessions (id, started_at, updated_at, date_key, alias, provider, model)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                [
                    &id,
                    &started_at,
                    &started_at,
                    &date_key,
                    alias,
                    provider,
                    model,
                ],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(id)
    }

    /// Append a record to a session.
    pub fn append_record(
        &self,
        session_id: &str,
        turn: Option<i64>,
        direction: TraceDirection,
        payload_json: &str,
        claude_session_id: Option<&str>,
    ) -> Result<(), AppError> {
        let timestamp = Utc::now().to_rfc3339();

        let next_index: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(record_index), 0) + 1 FROM records WHERE session_id = ?1",
                [session_id],
                |row| row.get(0),
            )
            .unwrap_or(1);

        self.conn
            .execute(
                "INSERT INTO records (session_id, record_index, turn, timestamp, direction, payload_json, claude_session_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    session_id,
                    next_index,
                    turn,
                    timestamp.clone(),
                    direction.as_str(),
                    payload_json,
                    claude_session_id,
                ],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        self.conn
            .execute(
                "UPDATE sessions SET updated_at = ?1, record_count = record_count + 1 WHERE id = ?2",
                [&timestamp, session_id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// List sessions ordered by most recent first, optionally filtered by date.
    pub fn list_sessions(
        &self,
        limit: i64,
        offset: i64,
        date: Option<&str>,
    ) -> Result<Vec<TraceSession>, AppError> {
        let sql = if date.is_some() {
            "SELECT id, started_at, updated_at, date_key, alias, provider, model, status, record_count, summary_json
             FROM sessions
             WHERE date_key = ?3
             ORDER BY updated_at DESC
             LIMIT ?1 OFFSET ?2"
        } else {
            "SELECT id, started_at, updated_at, date_key, alias, provider, model, status, record_count, summary_json
             FROM sessions
             ORDER BY updated_at DESC
             LIMIT ?1 OFFSET ?2"
        };

        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let sessions = if let Some(d) = date {
            stmt
                .query_map(rusqlite::params![limit, offset, d], Self::row_to_session)
        } else {
            stmt
                .query_map(rusqlite::params![limit, offset], Self::row_to_session)
        }
        .map_err(|e| AppError::Database(e.to_string()))?
        .collect::<Result<_, _>>()
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(sessions)
    }

    /// Count total sessions, optionally filtered by date.
    pub fn count_sessions(&self, date: Option<&str>) -> Result<i64, AppError> {
        let sql = if date.is_some() {
            "SELECT COUNT(*) FROM sessions WHERE date_key = ?1"
        } else {
            "SELECT COUNT(*) FROM sessions"
        };

        let count: i64 = if let Some(d) = date {
            self.conn
                .query_row(sql, [d], |row| row.get(0))
        } else {
            self.conn
                .query_row(sql, [], |row| row.get(0))
        }
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count)
    }

    /// Get a single session by id.
    pub fn get_session(&self, id: &str) -> Result<Option<TraceSession>, AppError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, started_at, updated_at, date_key, alias, provider, model, status, record_count, summary_json
                 FROM sessions WHERE id = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut sessions = stmt
            .query_map([id], Self::row_to_session)
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(sessions.next().transpose().map_err(|e| AppError::Database(e.to_string()))?)
    }

    /// Get all records for a session without pagination (for export).
    pub fn get_all_records(&self, session_id: &str) -> Result<Vec<TraceRecord>, AppError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT session_id, record_index, turn, timestamp, direction, payload_json, claude_session_id
                 FROM records WHERE session_id = ?1 ORDER BY record_index",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let records = stmt
            .query_map([session_id], Self::row_to_record)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<_, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(records)
    }

    /// Get records for a session, ordered by record_index, with pagination.
    pub fn get_records(
        &self,
        session_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TraceRecord>, AppError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT session_id, record_index, turn, timestamp, direction, payload_json, claude_session_id
                 FROM records WHERE session_id = ?1 ORDER BY record_index
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let records = stmt
            .query_map(rusqlite::params![session_id, limit, offset], Self::row_to_record)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<_, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(records)
    }

    /// Delete a session and its records (cascades).
    pub fn delete_session(&self, id: &str) -> Result<(), AppError> {
        self.conn
            .execute("DELETE FROM sessions WHERE id = ?1", [id])
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Clear all sessions and records.
    pub fn clear_all(&self) -> Result<(), AppError> {
        self.conn
            .execute("DELETE FROM sessions", [])
            .map_err(|e| AppError::Database(e.to_string()))?;
        // SQLite does not support TRUNCATE; DELETE with CASCADE handles records.
        // VACUUM reclaims disk space after bulk delete.
        self.conn
            .execute("VACUUM", [])
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Update session summary without changing status.
    pub fn update_summary(&self, id: &str, summary_json: &str) -> Result<(), AppError> {
        let updated_at = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "UPDATE sessions SET summary_json = ?1, updated_at = ?2 WHERE id = ?3",
                [summary_json, &updated_at, id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Update session status and summary.
    pub fn finalize_session(
        &self,
        id: &str,
        status: &str,
        summary_json: Option<&str>,
    ) -> Result<(), AppError> {
        let updated_at = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "UPDATE sessions SET status = ?1, summary_json = ?2, updated_at = ?3 WHERE id = ?4",
                [status, summary_json.unwrap_or(""), &updated_at, id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    fn row_to_session(row: &Row) -> Result<TraceSession, rusqlite::Error> {
        Ok(TraceSession {
            id: row.get(0)?,
            started_at: row.get(1)?,
            updated_at: row.get(2)?,
            date_key: row.get(3)?,
            alias: row.get(4)?,
            provider: row.get(5)?,
            model: row.get(6)?,
            status: row.get(7)?,
            record_count: row.get(8)?,
            summary_json: row.get(9)?,
        })
    }

    fn row_to_record(row: &Row) -> Result<TraceRecord, rusqlite::Error> {
        Ok(TraceRecord {
            session_id: row.get(0)?,
            record_index: row.get(1)?,
            turn: row.get(2)?,
            timestamp: row.get(3)?,
            direction: row.get(4)?,
            payload_json: row.get(5)?,
            claude_session_id: row.get(6)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_store() -> TraceStore {
        TraceStore::new(":memory:").unwrap()
    }

    #[test]
    fn test_create_session() {
        let store = create_test_store();
        let id = store.create_session("cl-kimi", "kimi", "kimi-k2.6").unwrap();
        assert!(!id.is_empty());

        let session = store.get_session(&id).unwrap().unwrap();
        assert_eq!(session.alias, "cl-kimi");
        assert_eq!(session.provider, "kimi");
        assert_eq!(session.model, "kimi-k2.6");
        assert_eq!(session.status, "active");
        assert_eq!(session.record_count, 0);
    }

    #[test]
    fn test_append_record() {
        let store = create_test_store();
        let id = store.create_session("cl-kimi", "kimi", "kimi-k2.6").unwrap();

        store
            .append_record(
                &id,
                Some(1),
                TraceDirection::Request,
                r#"{"model":"kimi-k2.6"}"#,
                None,
            )
            .unwrap();

        store
            .append_record(
                &id,
                Some(1),
                TraceDirection::Response,
                r#"{"content":"hello"}"#,
                None,
            )
            .unwrap();

        let session = store.get_session(&id).unwrap().unwrap();
        assert_eq!(session.record_count, 2);

        let records = store.get_records(&id, 100, 0).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].direction, "request");
        assert_eq!(records[1].direction, "response");
    }

    #[test]
    fn test_append_record_with_claude_session_id() {
        let store = create_test_store();
        let id = store.create_session("cl-kimi", "kimi", "kimi-k2.6").unwrap();

        store
            .append_record(
                &id,
                Some(1),
                TraceDirection::Request,
                r#"{"model":"kimi-k2.6"}"#,
                Some("dda157ab-1c1a-42d2-aec9-83b5d44789b9"),
            )
            .unwrap();

        let records = store.get_records(&id, 100, 0).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].claude_session_id, Some("dda157ab-1c1a-42d2-aec9-83b5d44789b9".to_string()));
    }

    #[test]
    fn test_delete_session_cascades() {
        let store = create_test_store();
        let id = store.create_session("cl-kimi", "kimi", "k1").unwrap();
        store
            .append_record(&id, Some(1), TraceDirection::Request, "{}", None)
            .unwrap();

        store.delete_session(&id).unwrap();

        assert!(store.get_session(&id).unwrap().is_none());
        assert!(store.get_records(&id, 100, 0).unwrap().is_empty());
    }

    #[test]
    fn test_list_sessions_pagination() {
        let store = create_test_store();
        let _id1 = store.create_session("cl-kimi", "kimi", "k1").unwrap();
        let _id2 = store.create_session("cl-mini", "minimax", "m1").unwrap();

        let all = store.list_sessions(10, 0, None).unwrap();
        assert_eq!(all.len(), 2);

        let page = store.list_sessions(1, 0, None).unwrap();
        assert_eq!(page.len(), 1);

        let page = store.list_sessions(10, 10, None).unwrap();
        assert_eq!(page.len(), 0);
    }

    #[test]
    fn test_list_sessions_by_date() {
        let store = create_test_store();
        let _id1 = store.create_session("cl-kimi", "kimi", "k1").unwrap();
        let _id2 = store.create_session("cl-mini", "minimax", "m1").unwrap();

        let today = Utc::now().date_naive().to_string();
        let filtered = store.list_sessions(10, 0, Some(&today)).unwrap();
        assert_eq!(filtered.len(), 2);

        let count = store.count_sessions(Some(&today)).unwrap();
        assert_eq!(count, 2);

        let count_all = store.count_sessions(None).unwrap();
        assert_eq!(count_all, 2);
    }

    #[test]
    fn test_finalize_session() {
        let store = create_test_store();
        let id = store.create_session("cl-kimi", "kimi", "k1").unwrap();

        store
            .finalize_session(&id, "complete", Some(r#"{"total_tokens":100}"#))
            .unwrap();

        let session = store.get_session(&id).unwrap().unwrap();
        assert_eq!(session.status, "complete");
        assert_eq!(
            session.summary_json,
            Some(r#"{"total_tokens":100}"#.to_string())
        );
    }
}
