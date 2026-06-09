use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::api::settings::Settings;
use crate::dao::SqliteDaoImpl;
use crate::trace::store::TraceStore;

/// 全局应用状态。所有 axum handler 通过 `State<AppState>` 注入。
/// 克隆廉价（内部 Arc），handler 拿到的都是同一个 dao。
#[derive(Clone)]
pub struct AppState {
    pub dao: Arc<Mutex<SqliteDaoImpl>>,
    pub settings: Arc<RwLock<Settings>>,
    pub trace_store: Arc<Mutex<TraceStore>>,
}

impl AppState {
    pub fn new(dao: SqliteDaoImpl, trace_store: TraceStore) -> Self {
        Self {
            dao: Arc::new(Mutex::new(dao)),
            settings: Arc::new(RwLock::new(Settings::default())),
            trace_store: Arc::new(Mutex::new(trace_store)),
        }
    }
}
