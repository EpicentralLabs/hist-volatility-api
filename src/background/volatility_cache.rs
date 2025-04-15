use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use tracing::{info, warn, error};
use crate::config::AppConfig;
use crate::routes::historical_volatility::{BirdeyeHistoricalPriceResponse, HistoricalPricePoint, calculate_volatility};

/// Cache for storing volatility data for different tokens
#[derive(Clone)]
pub struct VolatilityCache {
    /// Map of token address to (volatility, last_updated)
    cache: Arc<RwLock<HashMap<String, (f64, DateTime<Utc>)>>>,
    /// Configuration for API requests
    config: Arc<AppConfig>,
}

impl VolatilityCache {
    /// Create a new volatility cache
    pub fn new(config: AppConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(config),
        }
    }

    /// Get the current volatility for a token
    pub async fn get_volatility(&self, token_address: &str) -> Option<f64> {
        let cache = self.cache.read().await;
        cache.get(token_address).map(|(volatility, _)| *volatility)
    }

    /// Start the background task that updates volatility data every 60 seconds
    pub async fn start_background_task(&self) {
        let cache = Arc::clone(&self.cache);
        let config = Arc::clone(&self.config);
        
        tokio::spawn(async move {
            loop {
                // Sleep for 60 seconds
                tokio::time::sleep(Duration::from_secs(60)).await;
                
                // Update all cached tokens
                Self::update_all_tokens(&cache, &config).await;
            }
        });
    }

    /// Update volatility data for all tokens in the cache
    async fn update_all_tokens(
        cache: &Arc<RwLock<HashMap<String, (f64, DateTime<Utc>)>>>,
        config: &Arc<AppConfig>,
    ) {
        let token_addresses: Vec<String> = {
            let cache = cache.read().await;
            cache.keys().cloned().collect()
        };

        for token_address in token_addresses {
            if let Err(e) = Self::update_token(cache, config, &token_address).await {
                error!(token_address = %token_address, error = %e, "Failed to update token volatility");
            }
        }
    }

    /// Update volatility data for a specific token
    async fn update_token(
        cache: &Arc<RwLock<HashMap<String, (f64, DateTime<Utc>)>>>,
        config: &Arc<AppConfig>,
        token_address: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Calculate date range for 90-day rolling window
        let to_date = Utc::now();
        let from_date = to_date - ChronoDuration::days(90);

        // Fetch historical price data
        let response = Self::fetch_historical_prices(config, from_date, to_date, token_address).await?;
        
        // Process the response
        if let Some(data) = response.data {
            // Calculate volatility
            if let Some(volatility) = calculate_volatility(data.items) {
                // Update the cache
                let mut cache = cache.write().await;
                cache.insert(token_address.to_string(), (volatility, Utc::now()));
                
                info!(
                    token_address = %token_address,
                    volatility = %volatility,
                    "Updated token volatility"
                );
            } else {
                warn!(
                    token_address = %token_address,
                    "Not enough price data to calculate volatility"
                );
            }
        } else {
            warn!(
                token_address = %token_address,
                "No price data available"
            );
        }

        Ok(())
    }

    /// Fetch historical price data from Birdeye API
    async fn fetch_historical_prices(
        config: &Arc<AppConfig>,
        from_date: DateTime<Utc>,
        to_date: DateTime<Utc>,
        token_address: &str,
    ) -> Result<BirdeyeHistoricalPriceResponse, reqwest::Error> {
        // Convert DateTime objects to Unix timestamps for the API request
        let from_timestamp = from_date.timestamp();
        let to_timestamp = to_date.timestamp();

        // Construct the query string with required parameters
        let query = format!(
            "address={}&address_type=token&type=1D&time_from={}&time_to={}",
            token_address, from_timestamp, to_timestamp
        );
        let request_url = format!("{}?{}", config.birdeye_base_url, query);

        // Set up HTTP client with required headers
        let client = reqwest::Client::new();
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "X-API-KEY",
            reqwest::header::HeaderValue::from_str(&config.birdeye_api_key)
                .expect("Invalid API key format"),
        );
        headers.insert("x-chain", reqwest::header::HeaderValue::from_static("solana"));

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

    /// Add a token to the cache and immediately fetch its volatility
    pub async fn add_token(&self, token_address: String) -> Result<(), Box<dyn std::error::Error>> {
        let cache = Arc::clone(&self.cache);
        let config = Arc::clone(&self.config);
        
        Self::update_token(&cache, &config, &token_address).await?;
        
        Ok(())
    }
} 