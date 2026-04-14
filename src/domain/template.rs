use std::collections::HashMap;

/// Provider 模板，内置在程序中，包含默认环境变量和可选模型列表
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderTemplate {
    /// 模板唯一标识，如 "minimax"
    pub id: String,
    /// 模板显示名称，如 "MiniMax"
    pub name: String,
    /// 默认环境变量键值对
    pub default_env: HashMap<String, String>,
    /// 该 Provider 下支持的模型列表
    pub models: Vec<ModelTemplate>,
}

/// 模型模板，定义特定模型对默认环境变量的覆盖项
#[derive(Debug, Clone, PartialEq)]
pub struct ModelTemplate {
    /// 模型唯一标识，如 "MiniMax-M2.7-highspeed"
    pub id: String,
    /// 模型显示名称
    pub name: String,
    /// 需要覆盖或追加的环境变量键值对
    pub env_overrides: HashMap<String, String>,
}
