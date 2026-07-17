use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// API 错误。所有 handler 都返回 `Result<T, ApiError>`，由 IntoResponse 统一转为 JSON 错误响应。
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("资源不存在: {0}")]
    NotFound(String),

    /// 字段校验失败 (field, message)
    #[error("校验失败 [{field}]: {message}")]
    Validation { field: String, message: String },

    /// 资源冲突 (field, message) — 典型如 alias 重复
    #[error("冲突 [{field}]: {message}")]
    Conflict { field: String, message: String },

    #[error("内部错误: {0}")]
    Internal(String),
}

impl ApiError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }

    pub fn conflict(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Conflict {
            field: field.into(),
            message: message.into(),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    error: ErrorPayload<'a>,
}

#[derive(Serialize)]
struct ErrorPayload<'a> {
    code: &'a str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<&'a str>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, field) = match &self {
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND", None),
            ApiError::Validation { field, .. } => (
                StatusCode::BAD_REQUEST,
                "VALIDATION_ERROR",
                Some(field.as_str()),
            ),
            ApiError::Conflict { field, .. } => {
                (StatusCode::CONFLICT, "ALIAS_CONFLICT", Some(field.as_str()))
            }
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", None),
        };

        let body = ErrorBody {
            error: ErrorPayload {
                code,
                message: self.to_string(),
                field,
            },
        };
        (status, Json(body)).into_response()
    }
}
