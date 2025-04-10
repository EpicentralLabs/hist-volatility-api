use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use historical_volatility_api::config::AppConfig;
use historical_volatility_api::routes::historical_volatility::HistoricalVolatilityResponse;
use historical_volatility_api::routes::register_routes;
use once_cell::sync::Lazy;
use serde::Deserialize;
use tower::ServiceExt;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

//
// ----------- Global Setup -----------
//

static INIT: Lazy<()> = Lazy::new(|| {
    dotenvy::dotenv().ok();
});

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

//
// ----------- Test Helpers -----------
//

/// Helper to send a valid request to /historical_volatility
async fn send_valid_request(app: Router) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .uri("/historicalVolatility?fromDate=2024-12-31&toDate=2025-03-31&tokenAddress=So11111111111111111111111111111111111111112")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .expect("Should receive a response")
}

/// Helper to create a mock server returning the given JSON
async fn setup_mock_server(json_response: serde_json::Value) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json_response))
        .mount(&server)
        .await;
    server
}

//
// ----------- Happy Path Tests -----------
//

#[tokio::test]
async fn get_historical_volatility_returns_positive_value_with_mock() {
    let _ = *INIT;

    let fake_response = serde_json::json!({
        "success": true,
        "data": {
            "items": [
                { "unixTime": 1700000000, "value": 100.0 },
                { "unixTime": 1700008600, "value": 105.0 },
                { "unixTime": 1700017200, "value": 95.0 }
            ]
        }
    });

    let mock_server = setup_mock_server(fake_response).await;

    let config = AppConfig {
        birdeye_api_key: "dummy-key".to_string(),
        birdeye_base_url: mock_server.uri(),
        app_server_port: 8080
    };

    let app = register_routes(config);
    let response = send_valid_request(app).await;

    let status = response.status();
    let headers = response.headers().clone();
    let body = response.into_body();

    let historical_volatility_response: HistoricalVolatilityResponse =
        serde_json::from_slice(&to_bytes(body, usize::MAX).await.expect("should read body"))
            .expect("should parse JSON");

    assert_eq!(status, StatusCode::OK);
    assert!(
        historical_volatility_response.historical_volatility > 0.0,
        "Volatility should be > 0"
    );
    assert!(
        headers
            .get("content-type")
            .expect("Missing content-type")
            .to_str()
            .unwrap()
            .starts_with("application/json"),
        "Content-Type must be application/json"
    );
}

//
// ----------- Sad Path Tests -----------
//

#[tokio::test]
async fn get_historical_volatility_missing_api_key_returns_500() {
    let _ = *INIT;

    let fake_response = serde_json::json!({
        "success": false,
        "message": "Unauthorized"
    });

    let mock_server = setup_mock_server(fake_response).await;

    let config = AppConfig {
        birdeye_api_key: "".to_string(),
        birdeye_base_url: mock_server.uri(),
        app_server_port: 8080
    };

    let app = register_routes(config);
    let response = send_valid_request(app).await;

    let status = response.status();
    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("should read body");

    let error_response: ErrorResponse =
        serde_json::from_slice(&body_bytes).expect("should parse error response JSON");

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(error_response.error, "Internal Server Error");
    assert_eq!(error_response.message, "Something bad happened.");
}

#[tokio::test]
async fn get_historical_volatility_invalid_query_returns_400() {
    let _ = *INIT;

    let config = AppConfig {
        birdeye_api_key: "dummy".to_string(),
        birdeye_base_url: "https://public-api.birdeye.so/token_price/history".to_string(),
        app_server_port: 8080
    };

    let app = register_routes(config);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/historicalVolatility") // No query params at all
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("should have gotten a response");

    let status = response.status();
    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("should read body");

    let error_response: ErrorResponse =
        serde_json::from_slice(&body_bytes).expect("should parse error response JSON");

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_response.error, "Bad Request");

    assert_eq!(
        error_response.message,
        "Failed to deserialize query string: missing field `fromDate`"
    );
}

