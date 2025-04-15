use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

#[derive(Serialize, Deserialize, Debug)]
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
#[instrument(ret)]
pub async fn health_check() -> Json<HealthCheckResponse> {
    info!("Received health check request.");
    Json(HealthCheckResponse {
        message: "Server is running.".to_owned(),
    })
}
