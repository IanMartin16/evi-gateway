use actix_web::{HttpResponse, http::StatusCode};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Route not found: {0}")]
    RouteNotFound(String),

    #[error("Upstream timeout: {0}")]
    UpstreamTimeout(String),

    #[error("Upstream error: {0}")]
    UpstreamError(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub status: u16,
    pub code: String,
    pub message: String,
    pub request_id: Option<String>,
}

impl AppError {
    pub fn to_response(&self, request_id: Option<String>) -> HttpResponse {
        let (status, code) = match self {
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "FORBIDDEN"),
            AppError::RouteNotFound(_) => (StatusCode::NOT_FOUND, "ROUTE_NOT_FOUND"),
            AppError::UpstreamTimeout(_) => (StatusCode::GATEWAY_TIMEOUT, "UPSTREAM_TIMEOUT"),
            AppError::UpstreamError(_) => (StatusCode::BAD_GATEWAY, "UPSTREAM_ERROR"),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        };

        HttpResponse::build(status).json(ErrorResponse {
            status: status.as_u16(),
            code: code.to_string(),
            message: self.to_string(),
            request_id,
        })
    }
}

impl actix_web::ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        self.to_response(None)
    }
}