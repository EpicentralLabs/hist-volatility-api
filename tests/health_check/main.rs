use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use historical_volatility_api::routes::{health_check::HealthCheckResponse, register_routes};
use tower::ServiceExt;

#[tokio::test]
async fn health_check_returns_200_ok() {
    // Arrange
    let router = register_routes();

    // Act
    let response = router
        .oneshot(
            Request::builder()
                .uri("/health_check")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("should have gotten a response");

    let status = response.status();

    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("should have read body bytes");

    let health_response: HealthCheckResponse =
        serde_json::from_slice(&bytes).expect("should have deserialized JSON");

    // Assert
    assert_eq!(status, StatusCode::OK);
    assert_eq!(health_response.message, "Server is running.");
}
