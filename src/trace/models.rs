//! Trace data models.

/// A trace session representing one conversation through the proxy.
pub struct TraceSession {
    pub id: String,
}

/// A single record within a trace session.
pub struct TraceRecord {
    pub session_id: String,
    pub record_index: i64,
}
