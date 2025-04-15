use dotenvy::dotenv;
use historical_volatility_api::{
    background::volatility_cache::VolatilityCache,
    config::AppConfig, 
    routes::register_routes,
    state::AppState,
};

// TODO (Pen):
// - More documentation + tests, make sure everything is correct + how to actually use the API
#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt().init();

    let config = AppConfig::from_env().expect("Should have loaded config.");
    
    // Initialize the volatility cache
    let volatility_cache = VolatilityCache::new(config.clone());
    
    // Add SOL token to cache on startup
    match volatility_cache.add_token("So11111111111111111111111111111111111111112".to_string()).await {
        Ok(_) => tracing::info!("Added SOL token to volatility cache"),
        Err(e) => tracing::error!("Failed to add SOL token to cache: {}", e),
    }
    
    // Optionally add more tokens here
    // Example: USDC token
    match volatility_cache.add_token("LABSh5DTebUcUbEoLzXKCiXFJLecDFiDWiBGUU1GpxR".to_string()).await {
        Ok(_) => tracing::info!("Added USDC token to volatility cache"),
        Err(e) => tracing::error!("Failed to add LABS token to cache: {}", e),
    }
    
    // Start the background task that updates volatility data every 60 seconds
    volatility_cache.start_background_task().await;

    let state = AppState::new(config, volatility_cache);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", state.config.app_server_port))
        .await
        .unwrap();

    let addr = listener.local_addr().unwrap();
    tracing::info!("Listening on {}", addr);

    let app = register_routes(state);
    axum::serve(listener, app).await.unwrap();
}
