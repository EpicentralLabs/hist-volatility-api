use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use historical_volatility_api::config::AppConfig;
use historical_volatility_api::routes::{health_check::HealthCheckResponse, register_routes};
use tower::ServiceExt;

#[tokio::test]
async fn health_check_returns_200_ok() {
    // Arrange: Create router with dummy AppConfig
    let app = register_routes(AppConfig {
        birdeye_api_key: "DUMMY_KEY".to_string(),
        birdeye_base_url: "https://dummy.birdeye.api".to_string(),
        app_server_port: 8080
    });

    // Act: Send GET /health_check
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/healthCheck")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to get a response");

    // Extract status, headers, and body
    let status = response.status();
    let headers = response.headers().clone();
    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read body bytes");

    let health_response: HealthCheckResponse =
        serde_json::from_slice(&body_bytes).expect("Failed to deserialize JSON");

    // Assert
    assert_eq!(status, StatusCode::OK);
    assert!(
        headers
            .get("content-type")
            .expect("Content-Type header missing")
            .to_str()
            .unwrap()
            .starts_with("application/json"),
        "Content-Type should be application/json"
    );
    assert_eq!(
        health_response.message, "Server is running.",
        "Expected correct health check message"
    );
}
