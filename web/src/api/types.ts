// 与 Rust src/api/health.rs::HealthResponse 对齐（serde rename_all = "camelCase"）
export interface HealthResponse {
  status: 'ok' | 'error';
  version: string;
  dbPath: string;
}

// 与 Rust src/api/instances.rs::InstanceSummary 对齐
// 注：apiKey 仅在 detail 接口返回（S2+ 才会加）
// contextWindowEnabled 字段已废弃：context window 相关 env vars 现在由
// model template 的 env_overrides 字面量决定，instance 不再 toggle。
export interface Instance {
  id: string;
  templateId: string;
  alias: string;
  modelId: string;
  opencodeModelId: string;
  kvCacheEnabled: boolean;
}

// 与 Rust src/api/templates.rs::TemplateModelSummary 对齐
// contextWindow 字段已废弃：前端用 inferContextFromModelId(id) 从 model id
// 后缀（如 [1m]）推断上下文窗口大小；不再通过 API 字段传输。
export interface TemplateModel {
  id: string;
  name: string;
  opencodeModelId: string;
}

// 与 Rust src/api/templates.rs::TemplateSummary 对齐
export interface Template {
  id: string;
  displayName: string;
  opencodeProviderId: string;
  opencodeBaseUrl: string;
  /** @deprecated 旧字段，保留兼容；新代码用 models[].id */
  availableModels: string[];
  models: TemplateModel[];
  opencodeModels: string[];
}
