//! Trace data models.

use serde::{Deserialize, Serialize};

/// Direction of a trace record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceDirection {
    Request,
    Response,
}

impl std::fmt::Display for TraceDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TraceDirection::Request => write!(f, "request"),
            TraceDirection::Response => write!(f, "response"),
        }
    }
}

impl TraceDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            TraceDirection::Request => "request",
            TraceDirection::Response => "response",
        }
    }
}

/// A trace session representing one conversation through the proxy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSession {
    pub id: String,
    pub started_at: String,
    pub updated_at: String,
    pub date_key: String,
    pub alias: String,
    pub provider: String,
    pub model: String,
    pub status: String,
    pub record_count: i64,
    pub summary_json: Option<String>,
}

/// A single record within a trace session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceRecord {
    pub session_id: String,
    pub record_index: i64,
    pub turn: Option<i64>,
    pub timestamp: Option<String>,
    pub direction: String,
    pub payload_json: String,
    pub claude_session_id: Option<String>,
}
