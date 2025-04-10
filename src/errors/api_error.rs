use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

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
        let (status, error, message) = match self {
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
            ApiError::InvalidQuery(msg) => (StatusCode::BAD_REQUEST, "Bad Request", msg),
        };

        let body = ApiErrorResponse { error, message };

        (status, Json(body)).into_response()
    }
}
impl From<reqwest::Error> for ApiError {
    fn from(_err: reqwest::Error) -> Self {
        ApiError::InternalServerError
    }
}
