use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub message: String,
}
/// Health check endpoint.
///
/// Returns a `200 OK` status with a JSON payload indicating that the server is running.
///
/// # Response body
/// ```json
/// {
///   "message": "Server is running."
/// }
/// ```
///
/// Useful for uptime monitoring.
pub async fn health_check() -> Json<HealthCheckResponse> {
    Json(HealthCheckResponse {
        message: "Server is running.".to_owned(),
    })
}
