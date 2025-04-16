//! # Volatility Calculation Handler
//!
//! This module provides a single Axum handler for calculating historical volatility
//! based on token prices fetched from the Birdeye API.
//!
//! It is intended to be used **internally** in the backend, not as a standalone library.
//! It also contains data models and internal helpers necessary for this specific functionality.

use crate::config::AppConfig;
use crate::extractors::query_extractor::HistoricalVolatilityQuery;
use crate::{errors::api_error::ApiError, state::AppState};
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
#[derive(Debug, Deserialize, Clone)]
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
#[instrument(ret, err, skip(state))]
pub async fn get_historical_volatility(
    State(state): State<AppState>,
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
    if let Some(volatility) = state.volatility_cache.get_volatility(&query.token_address).await {
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
    if let Err(e) = state.volatility_cache.add_token(query.token_address.clone()).await {
        error!(
            token_address = %query.token_address,
            error = %e,
            "Failed to add token to volatility cache"
        );
        return Err(ApiError::InternalServerError);
    }

    // Get the newly calculated volatility from the cache
    let volatility = state.volatility_cache.get_volatility(&query.token_address).await
        .ok_or(ApiError::NotEnoughData)?;

    Ok(Json(HistoricalVolatilityResponse {
        historical_volatility: volatility,
    }))
}
/// Fetches historical token prices from the Birdeye public API.
///
/// # Notes
/// - Injects configuration (base URL, API key) from `AppConfig`.
#[allow(dead_code)]
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

/// Calculates the annualized volatility using the standard financial approach.
///
/// This function:
/// 1. Computes the logarithmic daily returns
/// 2. Calculates the standard deviation of these returns
/// 3. Annualizes the result (multiplies by √365, as crypto markets trade 24/7/365)
///
/// # Requirements
/// - At least two price points.
/// - Price points must be ordered chronologically.
///
/// # Example
/// For standard financial volatility, we:
/// 1. Calculate log returns: ln(P₁/P₀), ln(P₂/P₁), etc.
/// 2. Find the standard deviation of these returns
/// 3. Annualize by multiplying by √365 (for crypto markets)
///
/// instead of 252 days used for traditional stock markets
pub fn calculate_volatility(prices: Vec<HistoricalPricePoint>) -> Option<f64> {
    // Need at least 2 price points to calculate volatility
    if prices.len() < 2 {
        return None;
    }

    // Sort the prices by time (oldest first) to ensure chronological order
    let mut sorted_prices = prices;
    sorted_prices.sort_by_key(|point| point.unix_time);

    // Calculate the logarithmic daily returns
    let log_returns: Vec<f64> = sorted_prices.windows(2).map(|window| {
        let [previous, current] = window else {
            unreachable!("prices.windows(2) always yields exactly two items");
        };
        // Log return formula: ln(P₁/P₀)
        (current.value / previous.value).ln()
    }).collect();

    // We need at least one return to calculate standard deviation
    if log_returns.is_empty() {
        return None;
    }

    // Calculate the mean of log returns
    let mean = log_returns.iter().sum::<f64>() / log_returns.len() as f64;

    // Calculate the variance (average of squared differences from the mean)
    let variance = log_returns.iter()
        .map(|&return_value| (return_value - mean).powi(2))
        .sum::<f64>() / log_returns.len() as f64;

    // The daily volatility is the square root of the variance
    let daily_volatility = variance.sqrt();

    // Annualize the volatility using 365 days for crypto markets (which trade 24/7/365)
    // instead of 252 days used for traditional stock markets
    let annualized_volatility = daily_volatility * (365.0_f64).sqrt();
    
    // Convert to percentage for easier interpretation
    Some(annualized_volatility * 100.0)
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
        
        // With log returns: ln(105/100) ≈ 0.049, ln(95/105) ≈ -0.101
        // Mean of log returns: (0.049 + (-0.101))/2 = -0.026
        // Variance: ((0.049-(-0.026))² + (-0.101-(-0.026))²)/2 ≈ 0.0057
        // Daily volatility: √0.0057 ≈ 0.075
        // Annualized: 0.075 * √365 ≈ 4.24
        // As percentage: 4.24 * 100 = 424%
        assert!((result - 424.2).abs() < 1.0); // Allow some floating point error
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
        
        // With log returns: ln(180/200) ≈ -0.105
        // Mean of log returns: -0.105 (only one value)
        // Variance: 0 (only one value, so no deviation from mean)
        // Daily volatility: 0
        // Annualized: 0 (note: this is an edge case with only 2 points)
        // As percentage: 0
        
        // For single return case, the variance calculation will produce 0
        // This is an edge case in volatility calculation
        assert!(result.abs() < 1e-6);
    }

    #[test]
    fn test_calculate_volatility_with_more_realistic_data() {
        let prices = vec![
            HistoricalPricePoint { unix_time: 1, value: 100.0 },
            HistoricalPricePoint { unix_time: 2, value: 102.0 },
            HistoricalPricePoint { unix_time: 3, value: 99.0 },
            HistoricalPricePoint { unix_time: 4, value: 101.0 },
            HistoricalPricePoint { unix_time: 5, value: 103.0 },
            HistoricalPricePoint { unix_time: 6, value: 102.5 },
            HistoricalPricePoint { unix_time: 7, value: 103.5 },
        ];
        
        let result = calculate_volatility(prices).expect("Should calculate volatility");
        
        // This is a more realistic volatility test with several data points
        // For crypto with ~1-2% daily moves, annualized volatility using 365 days
        // would typically be higher than stock markets, often between 20-80%
        assert!(result > 15.0 && result < 85.0);
    }

    #[test]
    fn test_calculate_volatility_with_unsorted_data() {
        // Test with unsorted time data to ensure the function sorts correctly
        let prices = vec![
            HistoricalPricePoint { unix_time: 3, value: 95.0 },   // Note: out of order
            HistoricalPricePoint { unix_time: 1, value: 100.0 },  // Note: out of order
            HistoricalPricePoint { unix_time: 2, value: 105.0 },  // Note: out of order
        ];
        
        let result = calculate_volatility(prices).expect("Should calculate volatility");
        
        // Same expected result as test_calculate_volatility_with_three_prices
        assert!((result - 424.2).abs() < 1.0);
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
