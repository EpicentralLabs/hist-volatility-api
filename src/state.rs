use crate::config::AppConfig;
use crate::background::volatility_cache::VolatilityCache;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub volatility_cache: VolatilityCache,
}

impl AppState {
    pub fn new(config: AppConfig, volatility_cache: VolatilityCache) -> Self {
        Self {
            config,
            volatility_cache,
        }
    }
} 