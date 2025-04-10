use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::fmt;
use tracing::error;

#[derive(Debug)]
pub enum ApiError {
    InternalServerError,
    NotEnoughData,
    InvalidQuery(String),
}

#[derive(Serialize)]
struct ApiErrorResponse {
    error: &'static str,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error, message) = match &self {
            ApiError::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
                "Something bad happened.".to_owned(),
            ),
            ApiError::NotEnoughData => (
                StatusCode::BAD_REQUEST,
                "Bad Request",
                "Not enough price points to calculate volatility".to_owned(),
            ),
            ApiError::InvalidQuery(msg) => (StatusCode::BAD_REQUEST, "Bad Request", msg.clone()),
        };

        let body = ApiErrorResponse { error, message };

        let body_json = serde_json::to_string(&body)
            .unwrap_or_else(|_| "{\"error\":\"Serialization error\"}".to_string());

        error!(
            status = %status.as_u16(),
            json_body = %body_json,
            "Returning error response"
        );

        (status, Json(body)).into_response()
    }
}
impl From<reqwest::Error> for ApiError {
    fn from(_err: reqwest::Error) -> Self {
        ApiError::InternalServerError
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::InternalServerError => write!(f, "Internal server error"),
            ApiError::NotEnoughData => write!(f, "Not enough data"),
            ApiError::InvalidQuery(msg) => write!(f, "Invalid query: {}", msg),
        }
    }
}
