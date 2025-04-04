use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub message: String,
}
pub async fn health_check() -> Json<HealthCheckResponse> {
    Json(HealthCheckResponse {
        message: "Server is running.".to_owned(),
    })
}
