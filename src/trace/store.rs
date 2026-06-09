//! SQLite-backed trace storage.

use super::models::TraceSession;
use crate::domain::AppError;

/// Store for persisting trace sessions and records.
pub struct TraceStore;

impl TraceStore {
    pub fn new(_path: &str) -> Result<Self, AppError> {
        // TODO: implement in task 1.1
        Ok(Self)
    }

    pub fn create_session(
        &self,
        _alias: &str,
        _provider: &str,
        _model: &str,
    ) -> Result<String, AppError> {
        // TODO: implement in task 1.1
        Ok(String::new())
    }

    pub fn list_sessions(&self, _limit: i64, _offset: i64) -> Result<Vec<TraceSession>, AppError> {
        // TODO: implement in task 1.1
        Ok(vec![])
    }
}
