use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub birdeye_api_key: String,
    pub birdeye_base_url: String,
    pub app_server_port: u16
}

impl AppConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        let config = envy::from_env::<AppConfig>()?;

        if config.birdeye_api_key.trim().is_empty() {
            return Err(envy::Error::Custom(
                "BIRDEYE_API_KEY cannot be empty.".to_string(),
            ));
        }

        if config.birdeye_base_url.trim().is_empty() {
            return Err(envy::Error::Custom(
                "BIRDEYE_BASE_URL cannot be empty.".to_string(),
            ));
        }
        if config.app_server_port == 0 {
            return Err(envy::Error::Custom(
                "APP_SERVER_PORT cannot be 0.".to_string(),
            ));
        }

        Ok(config)
    }
}
