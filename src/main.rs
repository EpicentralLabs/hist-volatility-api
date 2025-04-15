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
