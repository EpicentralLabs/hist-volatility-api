use crate::{errors::api_error::ApiError, utils::custom_date_serde};
use axum::{
    extract::{FromRequestParts, Query},
    http::request::Parts,
    RequestPartsExt,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Query parameters for the volatility request.
#[derive(Deserialize)]
pub struct HistoricalVolatilityQuery {
    #[serde(with = "custom_date_serde")]
    pub from_date: DateTime<Utc>,
    #[serde(with = "custom_date_serde")]
    pub to_date: DateTime<Utc>,
    pub token_address: String,
}

impl<S> FromRequestParts<S> for HistoricalVolatilityQuery
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = parts
            .extract::<Query<HistoricalVolatilityQuery>>()
            .await
            .map_err(|err| ApiError::InvalidQuery(err.body_text()))?;
        Ok(query)
    }
}
