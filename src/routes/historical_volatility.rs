//! # Volatility Calculation Handler
//!
//! This module provides a single Axum handler for calculating historical volatility
//! based on token prices fetched from the Birdeye API.
//!
//! It is intended to be used **internally** in the backend, not as a standalone library.
//! It also contains data models and internal helpers necessary for this specific functionality.

use crate::extractors::query_extractor::HistoricalVolatilityQuery;
use crate::{config::AppConfig, errors::api_error::ApiError, background::volatility_cache::VolatilityCache};
use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue},
    Json,
};
use chrono::{DateTime, Utc};
use reqwest::header::ACCEPT;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, error};

//
// ----------- Data Structures -----------
//

/// Response structure returned by the API after successful volatility calculation.
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalVolatilityResponse {
    pub historical_volatility: f64,
}

/// Raw structure of the response returned by the Birdeye API.
#[derive(Debug, Deserialize)]
pub struct BirdeyeHistoricalPriceResponse {
    pub data: Option<HistoricalPriceData>,
    pub success: bool,
    pub message: Option<String>,
}

/// Nested `data` field inside the Birdeye response.
#[derive(Debug, Deserialize)]
pub struct HistoricalPriceData {
    pub items: Vec<HistoricalPricePoint>,
}

/// Represents a single historical price point.
#[derive(Debug, Deserialize)]
pub struct HistoricalPricePoint {
    #[serde(rename = "unixTime")]
    pub unix_time: i64,
    pub value: f64,
}

/// Internal representation of Birdeye response, abstracting success and failure.
#[derive(Debug)]
pub enum BirdeyeResponse {
    Success(HistoricalPriceData),
    Failure(String),
}

//
// ----------- Conversions -----------
//

impl From<BirdeyeHistoricalPriceResponse> for BirdeyeResponse {
    fn from(raw: BirdeyeHistoricalPriceResponse) -> Self {
        if raw.success {
            if let Some(data) = raw.data {
                BirdeyeResponse::Success(data)
            } else {
                BirdeyeResponse::Failure("Missing data in successful Birdeye response.".to_string())
            }
        } else {
            let message = raw.message.unwrap_or_else(|| "Unknown error".to_string());
            BirdeyeResponse::Failure(message)
        }
    }
}

//
// ----------- Handlers and Logic -----------
//

/// Axum handler that fetches historical prices from Birdeye and calculates volatility.
///
/// # Errors
/// - Returns `400 Bad Request` for invalid user input (wrong address or wrong date format).
/// - Returns `500 Internal Server Error` for unexpected Birdeye failures or internal issues.
#[instrument(ret, err, skip(config, volatility_cache))]
pub async fn get_historical_volatility(
    State(config): State<AppConfig>,
    State(volatility_cache): State<VolatilityCache>,
    query: HistoricalVolatilityQuery,
) -> Result<Json<HistoricalVolatilityResponse>, ApiError> {
    // Log the incoming request parameters
    info!(
        from_date = %query.from_date,
        to_date = %query.to_date,
        token_address = %query.token_address,
        "Received historical volatility request."
    );

    // Check if we have cached volatility data for this token
    if let Some(volatility) = volatility_cache.get_volatility(&query.token_address).await {
        info!(
            token_address = %query.token_address,
            volatility = %volatility,
            "Returning cached volatility data"
        );
        
        return Ok(Json(HistoricalVolatilityResponse {
            historical_volatility: volatility,
        }));
    }

    // If not in cache, add it to the cache and calculate volatility
    if let Err(e) = volatility_cache.add_token(query.token_address.clone()).await {
        error!(
            token_address = %query.token_address,
            error = %e,
            "Failed to add token to volatility cache"
        );
        return Err(ApiError::InternalServerError);
    }

    // Get the newly calculated volatility from the cache
    let volatility = volatility_cache.get_volatility(&query.token_address).await
        .ok_or(ApiError::NotEnoughData)?;

    Ok(Json(HistoricalVolatilityResponse {
        historical_volatility: volatility,
    }))
}
/// Fetches historical token prices from the Birdeye public API.
///
/// # Notes
/// - Injects configuration (base URL, API key) from `AppConfig`.
async fn make_birdeye_request(
    config: &AppConfig,
    from_date: DateTime<Utc>,
    to_date: DateTime<Utc>,
    token_address: &str,
) -> Result<BirdeyeHistoricalPriceResponse, reqwest::Error> {
    // Convert DateTime objects to Unix timestamps for the API request
    let from_timestamp = from_date.timestamp();
    let to_timestamp = to_date.timestamp();

    // Construct the query string with required parameters:
    // - address: The token address to fetch prices for
    // - address_type: Set to "token" to indicate we're querying a token
    // - type: Set to "1D" to get daily price data
    // - time_from: Start timestamp
    // - time_to: End timestamp
    let query = format!(
        "address={}&address_type=token&type=1D&time_from={}&time_to={}",
        token_address, from_timestamp, to_timestamp
    );
    let request_url = format!("{}?{}", config.birdeye_base_url, query);

    // Set up HTTP client with required headers
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(
        "X-API-KEY",
        HeaderValue::from_str(&config.birdeye_api_key).expect("Invalid API key format"),
    );
    headers.insert("x-chain", HeaderValue::from_static("solana"));

    // Make the HTTP request and parse the JSON response
    let response = client
        .get(request_url)
        .headers(headers)
        .send()
        .await?
        .json::<BirdeyeHistoricalPriceResponse>()
        .await?;

    Ok(response)
}

/// Calculates the average absolute daily fluctuation (volatility).
///
/// # Requirements
/// - At least two price points.
/// - Price points must be ordered chronologically.
///
/// # Example
/// ```
/// // For prices [100, 105, 95]
/// // Daily changes: +5, -10
/// // Volatility = (|5| + |10|) / 2 = 7.5
/// ```
pub fn calculate_volatility(prices: Vec<HistoricalPricePoint>) -> Option<f64> {
    // Need at least 2 price points to calculate volatility
    if prices.len() < 2 {
        return None;
    }

    // Calculate the sum of absolute daily price changes
    // This uses a sliding window of 2 elements to compare consecutive prices
    let total_abs_fluctuation = prices.windows(2).fold(0.0, |acc, window| {
        let [previous, next] = window else {
            unreachable!("prices.windows(2) always yields exactly two items");
        };
        // Add the absolute difference between consecutive prices
        acc + (next.value - previous.value).abs()
    });

    // Calculate the average daily fluctuation by dividing the total by (n-1)
    // where n is the number of price points
    // We use (n-1) because with n price points, we have (n-1) daily changes
    let avg_fluctuation = total_abs_fluctuation / (prices.len() - 1) as f64;
    Some(avg_fluctuation)
}

//
// ----------- Tests -----------
//

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use dotenvy::dotenv;
    use once_cell::sync::Lazy;

    static INIT: Lazy<()> = Lazy::new(|| {
        dotenv().ok();
    });

    fn test_config() -> AppConfig {
        AppConfig {
            birdeye_api_key: std::env::var("BIRDEYE_API_KEY")
                .unwrap_or_else(|_| "dummy".to_string()),
            birdeye_base_url: std::env::var("BIRDEYE_BASE_URL").unwrap_or_else(|_| {
                "https://public-api.birdeye.so/token_price/history".to_string()
            }),
            app_server_port: 8080,
        }
    }

    fn from_and_to_dates(days: i64) -> (DateTime<Utc>, DateTime<Utc>) {
        let to = Utc::now().date_naive() - Duration::days(1);
        let from = to - Duration::days(days - 1);
        (
            from.and_hms_opt(0, 0, 0).unwrap().and_utc(),
            to.and_hms_opt(0, 0, 0).unwrap().and_utc(),
        )
    }

    #[test]
    fn test_calculate_volatility_with_three_prices() {
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
        let result = calculate_volatility(prices).expect("Should calculate volatility");
        assert!((result - 7.5).abs() < 1e-6);
    }

    #[test]
    fn test_calculate_volatility_with_two_prices() {
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
        assert!((result - 20.0).abs() < 1e-6);
    }

    #[test]
    fn test_calculate_volatility_with_insufficient_prices() {
        let prices = vec![HistoricalPricePoint {
            unix_time: 1,
            value: 100.0,
        }];
        assert!(calculate_volatility(prices).is_none());
    }

    #[tokio::test]
    #[ignore = "Expensive - real HTTP call"]
    async fn test_make_birdeye_request_real() {
        let _ = *INIT;
        let config = test_config();
        let (from_date, to_date) = from_and_to_dates(10);

        let response = make_birdeye_request(
            &config,
            from_date,
            to_date,
            "So11111111111111111111111111111111111111112",
        )
        .await
        .expect("Birdeye request should succeed");

        let data = response.data.expect("Expected data field present");
        assert_eq!(data.items.len(), 10);
    }
}
