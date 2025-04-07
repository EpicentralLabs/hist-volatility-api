use dotenvy::dotenv;
use historical_volatility_api::{config::AppConfig, routes::register_routes};

// TODO (Pen):
// - Logging
// - Be careful what you log when it comes to secrets!
#[tokio::main]
async fn main() {
    dotenv().ok();
    let config = AppConfig::from_env().expect("should have loaded config.");

    let app = register_routes(config);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    
    println!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
