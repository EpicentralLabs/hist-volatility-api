use dotenvy::dotenv;
use historical_volatility_api::{config::AppConfig, routes::register_routes};

// TODO (Pen):
// - Logging
// - Be careful what you log when it comes to secrets!
// - More documentation + tests, make sure everything is correct + how to actually use the API
// - camelCase in queries
#[tokio::main]
async fn main() {
    dotenv().ok();
    let config = AppConfig::from_env().expect("Should have loaded config.");

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.app_server_port))
        .await
        .unwrap();

    let app = register_routes(config);

    println!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
