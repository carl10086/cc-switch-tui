// 与 Rust src/api/health.rs::HealthResponse 对齐（serde rename_all = "camelCase"）
export interface HealthResponse {
  status: 'ok' | 'error';
  version: string;
  dbPath: string;
}

// 与 Rust src/api/instances.rs::InstanceSummary 对齐
// 注：apiKey 仅在 detail 接口返回（S2+ 才会加）
export interface Instance {
  id: string;
  templateId: string;
  alias: string;
  modelId: string;
  opencodeModelId: string;
  kvCacheEnabled: boolean;
}

// 与 Rust src/api/templates.rs::TemplateModelSummary 对齐
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
}
