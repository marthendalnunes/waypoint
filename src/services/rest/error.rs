use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RestError {
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    #[error("Resource not found: {0}")]
    NotFound(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl RestError {
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::InvalidParams(message.into())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidParams(_) => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::InvalidParams(_) => "invalid_params",
            Self::NotFound(_) => "not_found",
            Self::Internal(_) => "internal_error",
        }
    }
}

impl From<crate::services::rest::state::ResourceReadError> for RestError {
    fn from(value: crate::services::rest::state::ResourceReadError) -> Self {
        match value {
            crate::services::rest::state::ResourceReadError::InvalidParams(message) => {
                Self::InvalidParams(message)
            },
            crate::services::rest::state::ResourceReadError::NotFound(message) => {
                Self::NotFound(message)
            },
            crate::services::rest::state::ResourceReadError::Internal(message) => {
                Self::Internal(message)
            },
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

impl IntoResponse for RestError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body =
            ErrorEnvelope { error: ErrorBody { code: self.code(), message: self.to_string() } };

        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};

    use super::RestError;

    #[tokio::test]
    async fn status_mapping_is_correct() {
        let bad = RestError::invalid_params("bad").into_response();
        assert_eq!(bad.status(), StatusCode::BAD_REQUEST);

        let not_found = RestError::NotFound("missing".to_string()).into_response();
        assert_eq!(not_found.status(), StatusCode::NOT_FOUND);

        let internal = RestError::Internal("boom".to_string()).into_response();
        assert_eq!(internal.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn error_schema_is_stable() {
        let response = RestError::invalid_params("invalid fid").into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(value["error"]["code"], "invalid_params");
        assert!(value["error"]["message"].as_str().unwrap().contains("invalid fid"));
    }
}
