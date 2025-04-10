use dotenvy::dotenv;
use historical_volatility_api::{config::AppConfig, routes::register_routes};

// TODO (Pen):
// - More documentation + tests, make sure everything is correct + how to actually use the API
#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt().init();

    let config = AppConfig::from_env().expect("Should have loaded config.");

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.app_server_port))
        .await
        .unwrap();

    let addr = listener.local_addr().unwrap();
    tracing::info!("Listening on {}", addr);

    let app = register_routes(config);
    axum::serve(listener, app).await.unwrap();
}
