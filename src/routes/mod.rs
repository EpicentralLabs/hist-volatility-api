use axum::{routing::get, Router};
use historical_volatility::get_historical_volatility;
use health_check::health_check;

pub mod historical_volatility;
pub mod health_check;

pub fn register_routes() -> Router {
    Router::new()
    .route("/volatility", get(get_historical_volatility))
    .route("/health_check", get(health_check))
}