use crate::config::AppConfig;
use crate::background::volatility_cache::VolatilityCache;
use axum::{
    http::{Response, StatusCode},
    routing::get,
    Router,
};
use health_check::health_check;
use historical_volatility::get_historical_volatility;
use tower_http::{
    catch_panic::CatchPanicLayer,
    trace::{DefaultOnRequest, TraceLayer},
};
use tracing::Level;

pub mod health_check;
pub mod historical_volatility;

pub fn register_routes(config: AppConfig, volatility_cache: VolatilityCache) -> Router {
    // TODO (Pen): I'll need to think about the CORS.
    // let cors = CorsLayer::new()
    // .allow_methods(Any)
    // .allow_origin(Any)
    // .allow_headers(Any);

    Router::new()
        .route("/historicalVolatility", get(get_historical_volatility))
        .route("/healthCheck", get(health_check))
        .with_state(config)
        .with_state(volatility_cache)
        .layer(CatchPanicLayer::custom(|_err| panic_handler()))
        .layer(TraceLayer::new_for_http().on_request(DefaultOnRequest::new().level(Level::INFO)))
    // .layer(cors)
}

fn panic_handler() -> Response<String> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("content-type", "application/json")
        .body("{\"error\": \"Something bad happened.\"}".to_string())
        .unwrap()
}
