use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use historical_volatility_api::config::AppConfig;
use historical_volatility_api::routes::historical_volatility::HistoricalVolatilityResponse;
use historical_volatility_api::routes::register_routes;
use once_cell::sync::Lazy;
use tower::ServiceExt;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

// Load .env ONCE globally
static INIT: Lazy<()> = Lazy::new(|| {
    dotenvy::dotenv().ok();
});

/// Helper: send a request to /historical_volatility
async fn send_valid_request(app: Router) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .uri("/historical_volatility?from_date=2024-12-31&to_date=2025-03-31&token_address=So11111111111111111111111111111111111111112")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .expect("should have gotten a response")
}

/// ✅ Happy path: should succeed using a Wiremock server
#[tokio::test]
async fn get_historical_volatility_returns_positive_value_with_mock() {
    let _ = *INIT;

    let mock_server = MockServer::start().await;

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

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fake_response))
        .mount(&mock_server)
        .await;

    let config = AppConfig {
        birdeye_api_key: "dummy-key".to_string(),
        birdeye_base_url: mock_server.uri(),
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

/// ❌ Missing API key (empty string simulates it)
#[tokio::test]
async fn get_historical_volatility_missing_api_key_returns_500() {
    let _ = *INIT;

    let config = AppConfig {
        birdeye_api_key: "".to_string(),
        birdeye_base_url: "https://public-api.birdeye.so/defi/history_price".to_string(),
    };

    let app = register_routes(config);
    let response = send_valid_request(app).await;

    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("should read body");
    let body_text = String::from_utf8_lossy(&body);

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        body_text.trim(),
        "{\"error\": \"Something bad happened.\"}",
        "Expected panic handler JSON"
    );
}

/// ❌ Invalid query parameters (missing fields)
#[tokio::test]
async fn get_historical_volatility_invalid_query_returns_400() {
    let _ = *INIT;

    let config = AppConfig {
        birdeye_api_key: "dummy".to_string(),
        birdeye_base_url: "https://public-api.birdeye.so/defi/history_price".to_string(),
    };

    let app = register_routes(config);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/historical_volatility") // No query params
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("should have gotten a response");

    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("should read body");
    let body_text = String::from_utf8_lossy(&body);

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body_text.to_lowercase().contains("invalid")
            || body_text.to_lowercase().contains("missing"),
        "Expected error mentioning invalid or missing fields, got: {}",
        body_text
    );
}
