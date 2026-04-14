use chrono::{DateTime, Utc};

/// 用户创建的 Provider 实例，对应一个具体的模板和模型配置
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderInstance {
    /// 实例唯一标识，格式为 "template_id-model_id"
    pub id: String,
    /// 关联的 Provider 模板 ID
    pub template_id: String,
    /// 关联的 Model 模板 ID
    pub model_id: String,
    /// 用户输入的 API Key
    pub api_key: String,
    /// 实例创建时间
    pub created_at: DateTime<Utc>,
}
