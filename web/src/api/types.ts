// 与 Rust src/api/health.rs::HealthResponse 对齐（serde rename_all = "camelCase"）
export interface HealthResponse {
  status: 'ok' | 'error';
  version: string;
  dbPath: string;
}
