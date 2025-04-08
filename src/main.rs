use dotenvy::dotenv;
use historical_volatility_api::{config::AppConfig, routes::register_routes};

// TODO (Pen):
// - Logging
// - Be careful what you log when it comes to secrets!
// - More documentation + tests, make sure everything is correct + how to actually use the API
// - What happens when you send invalid data to birdeye? And what does this API send back? (like when you input an invalid token address)
#[tokio::main]
async fn main() {
    dotenv().ok();
    let config = AppConfig::from_env().expect("should have loaded config.");

    let app = register_routes(config);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    
    println!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
