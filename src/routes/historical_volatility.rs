//! # Historical Volatility API
//!
//! This module provides:
//! - A function to fetch historical token prices from Birdeye API.
//! - A function to calculate simple volatility based on daily price fluctuations.
//! - Supporting data structures for deserializing Birdeye API responses.

use crate::extractors::query_extractor::HistoricalVolatilityQuery;
use crate::{config::AppConfig, errors::api_error::ApiError};
use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue},
    Json,
};
use chrono::{DateTime, Utc};
use reqwest::header::ACCEPT;
use serde::{Deserialize, Serialize};

/// Response structure sent back from our API.
#[derive(Deserialize, Serialize)]
pub struct HistoricalVolatilityResponse {
    pub historical_volatility: f64,
}

/// Actual response structure from the Birdeye public API.
#[derive(Deserialize)]
pub struct BirdeyeHistoricalPriceResponse {
    pub data: HistoricalPriceData,
    pub success: bool,
}

/// Nested data field inside the Birdeye response.
#[derive(Deserialize)]
pub struct HistoricalPriceData {
    pub items: Vec<HistoricalPricePoint>,
}

/// Represents a single price point at a timestamp.
#[derive(Deserialize)]
pub struct HistoricalPricePoint {
    #[serde(rename = "unixTime")]
    pub unix_time: i64,
    pub value: f64,
}

/// Axum handler that returns historical volatility based on Birdeye prices.
///
/// # Errors
/// - Returns `502 Bad Gateway` if Birdeye cannot be reached.
/// - Returns `400 Bad Request` if not enough price points to calculate volatility.
pub async fn get_historical_volatility(
    State(config): State<AppConfig>,
    query: HistoricalVolatilityQuery,
) -> Result<Json<HistoricalVolatilityResponse>, ApiError> {
    let birdeye_response = make_birdeye_request(
        &config,
        query.from_date,
        query.to_date,
        &query.token_address,
    )
    .await?;

    let historical_volatility =
        calculate_volatility(birdeye_response.data.items).ok_or(ApiError::NotEnoughData)?;

    Ok(Json(HistoricalVolatilityResponse {
        historical_volatility,
    }))
}

/// Fetch historical token prices from Birdeye's public API.
///
/// # Notes
/// - Configuration (base URL, API key) is injected through `AppConfig`.
async fn make_birdeye_request(
    config: &AppConfig,
    from_date: DateTime<Utc>,
    to_date: DateTime<Utc>,
    token_address: &str,
) -> Result<BirdeyeHistoricalPriceResponse, reqwest::Error> {
    let from_date_timestamp = from_date.timestamp();
    let to_date_timestamp = to_date.timestamp();
    let query = format!(
        "address={}&address_type=token&type=1D&time_from={}&time_to={}",
        token_address, from_date_timestamp, to_date_timestamp
    );

    let request_url = format!("{}?{}", config.birdeye_base_url, query);

    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(
        "X-API-KEY",
        HeaderValue::from_str(&config.birdeye_api_key).expect("Invalid API key format"),
    );
    headers.insert("x-chain", HeaderValue::from_static("solana"));

    let response = client
        .get(request_url)
        .headers(headers)
        .send()
        .await?
        .json::<BirdeyeHistoricalPriceResponse>()
        .await?;

    Ok(response)
}

/// Calculate volatility as the average of absolute daily fluctuations.
///
/// # Assumptions:
/// - Prices must be ordered by ascending date (earliest first).
///
/// # Example:
/// ```
/// // For prices [100, 105, 95]
/// // Daily changes: +5, -10
/// // Volatility = (|5| + |10|) / 2 = 7.5
/// ```
fn calculate_volatility(prices: Vec<HistoricalPricePoint>) -> Option<f64> {
    if prices.len() < 2 {
        return None;
    }

    let total_absolute_fluctuation = prices.windows(2).fold(0.0, |acc, window| {
        let [previous, next] = window else {
            unreachable!() // the array will always have at least 2 elements (checks at the start of the function)
        };
        acc + (next.value - previous.value).abs()
    });

    let average_fluctuation = total_absolute_fluctuation / (prices.len() - 1) as f64;
    Some(average_fluctuation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use dotenvy::dotenv;
    use once_cell::sync::Lazy;

    static INIT: Lazy<()> = Lazy::new(|| {
        dotenv().ok();
    });

    /// Test helper to create a fake AppConfig for tests.
    fn test_config() -> AppConfig {
        AppConfig {
            birdeye_api_key: std::env::var("BIRDEYE_API_KEY")
                .unwrap_or_else(|_| "dummy".to_string()),
            birdeye_base_url: std::env::var("BIRDEYE_BASE_URL").unwrap(),
        }
    }

    fn from_and_to_dates(days: i64) -> (DateTime<Utc>, DateTime<Utc>) {
        let to_date = Utc::now().date_naive() - Duration::days(1);
        let from_date = to_date - Duration::days(days - 1);
        (
            from_date.and_hms_opt(0, 0, 0).unwrap().and_utc(),
            to_date.and_hms_opt(0, 0, 0).unwrap().and_utc(),
        )
    }

    #[test]
    fn test_volatility_with_three_prices() {
        let prices = vec![
            HistoricalPricePoint {
                unix_time: 1,
                value: 100.0,
            },
            HistoricalPricePoint {
                unix_time: 2,
                value: 105.0,
            },
            HistoricalPricePoint {
                unix_time: 3,
                value: 95.0,
            },
        ];

        let result = calculate_volatility(prices).expect("should have calculated volatility.");
        assert!((result - 7.5).abs() < 1e-6, "Expected 7.5, got {}", result);
    }

    #[test]
    fn test_volatility_with_two_prices() {
        let prices = vec![
            HistoricalPricePoint {
                unix_time: 1,
                value: 200.0,
            },
            HistoricalPricePoint {
                unix_time: 2,
                value: 180.0,
            },
        ];

        let result = calculate_volatility(prices).expect("Should calculate volatility");
        assert!(
            (result - 20.0).abs() < 1e-6,
            "Expected 20.0, got {}",
            result
        );
    }

    #[test]
    fn test_volatility_with_large_set_of_prices() {
        let prices = (0..11)
            .map(|i| HistoricalPricePoint {
                unix_time: i,
                value: 100.0 + i as f64,
            })
            .collect::<Vec<_>>();

        let result = calculate_volatility(prices).expect("Should calculate volatility");
        assert!((result - 1.0).abs() < 1e-6, "Expected 1.0, got {}", result);
    }

    #[test]
    fn test_volatility_not_enough_prices() {
        let prices = vec![HistoricalPricePoint {
            unix_time: 1,
            value: 100.0,
        }];
        let result = calculate_volatility(prices);
        assert!(result.is_none(), "Expected None for too few prices");
    }

    #[tokio::test]
    #[ignore = "too expensive to run every time"]
    async fn test_make_birdeye_request_10_days() {
        let _ = *INIT;
        let (from_date, to_date) = from_and_to_dates(10);
        let config = test_config();

        let response = make_birdeye_request(
            &config,
            from_date,
            to_date,
            "So11111111111111111111111111111111111111112",
        )
        .await
        .expect("Request should succeed");

        assert_eq!(response.data.items.len(), 10, "Expected 10 price points");
    }

    #[tokio::test]
    #[ignore = "too expensive to run every time"]
    async fn test_make_birdeye_request_30_days() {
        let _ = *INIT;
        let (from_date, to_date) = from_and_to_dates(30);
        let config = test_config();

        let response = make_birdeye_request(
            &config,
            from_date,
            to_date,
            "So11111111111111111111111111111111111111112",
        )
        .await
        .expect("Request should succeed");

        assert_eq!(response.data.items.len(), 30, "Expected 30 price points");
    }

    #[tokio::test]
    #[ignore = "too expensive to run every time"]
    async fn test_make_birdeye_request_90_days() {
        let _ = *INIT;
        let (from_date, to_date) = from_and_to_dates(90);
        let config = test_config();

        let response = make_birdeye_request(
            &config,
            from_date,
            to_date,
            "So11111111111111111111111111111111111111112",
        )
        .await
        .expect("Request should succeed");

        assert_eq!(response.data.items.len(), 90, "Expected 90 price points");
    }
}
