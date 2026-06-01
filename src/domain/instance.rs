use chrono::{DateTime, Utc};

use super::error::AppError;

/// 用户创建的 Provider 实例，对应一个具体的模板和模型配置
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderInstance {
    /// 实例唯一标识，格式为 "template_id-alias"
    /// （注：model_id 不在 id 中，改 model 不会破坏主键稳定性）
    pub id: String,
    /// 关联的 Provider 模板 ID
    pub template_id: String,
    /// 关联的 Model 模板 ID
    pub model_id: String,
    /// 用户输入的 API Key
    pub api_key: String,
    /// 实例创建时间
    pub created_at: DateTime<Utc>,
    /// 实例别名
    pub alias: String,
    /// OpenCode model ID
    pub opencode_model_id: String,
    /// 是否启用 KV Cache 优化（默认 false）
    pub kv_cache_enabled: bool,
}

/// alias 校验：只能小写字母、数字、-、_，长度 1-32，不能为空，不能有空白或大写
pub fn validate_alias(alias: &str) -> Result<(), AppError> {
    if alias.is_empty() {
        return Err(AppError::InvalidAlias("alias 不能为空".to_string()));
    }
    if alias.len() > 32 {
        return Err(AppError::InvalidAlias("alias 长度不能超过 32 字符".to_string()));
    }
    if !alias
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        return Err(AppError::InvalidAlias(
            "alias 只能包含小写字母、数字、-、_".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_alias_accepts_lowercase_alnum_dash_underscore() {
        assert!(validate_alias("cl-mini").is_ok());
        assert!(validate_alias("cl_mini").is_ok());
        assert!(validate_alias("cl-123").is_ok());
        assert!(validate_alias("abc").is_ok());
        assert!(validate_alias("a-b_c-d_e").is_ok());
    }

    #[test]
    fn test_validate_alias_rejects_uppercase() {
        assert!(matches!(
            validate_alias("cl-Mini"),
            Err(AppError::InvalidAlias(_))
        ));
        assert!(matches!(
            validate_alias("CL-mini"),
            Err(AppError::InvalidAlias(_))
        ));
    }

    #[test]
    fn test_validate_alias_rejects_whitespace() {
        assert!(matches!(
            validate_alias("cl mini"),
            Err(AppError::InvalidAlias(_))
        ));
        assert!(matches!(
            validate_alias(" cl-mini"),
            Err(AppError::InvalidAlias(_))
        ));
        assert!(matches!(
            validate_alias("cl-mini "),
            Err(AppError::InvalidAlias(_))
        ));
        assert!(matches!(
            validate_alias("cl\t-mini"),
            Err(AppError::InvalidAlias(_))
        ));
    }

    #[test]
    fn test_validate_alias_rejects_empty() {
        assert!(matches!(validate_alias(""), Err(AppError::InvalidAlias(_))));
    }

    #[test]
    fn test_validate_alias_rejects_too_long() {
        let long = "a".repeat(33);
        assert!(matches!(
            validate_alias(&long),
            Err(AppError::InvalidAlias(_))
        ));
        let max = "a".repeat(32);
        assert!(validate_alias(&max).is_ok());
    }
}
