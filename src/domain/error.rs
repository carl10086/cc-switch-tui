use thiserror::Error;

/// 应用统一的错误类型
#[derive(Error, Debug, PartialEq)]
pub enum AppError {
    #[error("实例已存在: {0}")]
    InstanceAlreadyExists(String),

    #[error("实例不存在: {0}")]
    InstanceNotFound(String),

    #[error("模板不存在: {0}")]
    TemplateNotFound(String),

    #[error("模型不存在: {0}")]
    ModelNotFound(String),
}
