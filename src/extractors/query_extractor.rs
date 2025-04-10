use crate::{errors::api_error::ApiError, utils::custom_date_serde};
use axum::{
    extract::{FromRequestParts, Query},
    http::request::Parts,
    RequestPartsExt,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use tracing::{error, info};

/// Query parameters for the volatility request.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
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
        match parts.extract::<Query<HistoricalVolatilityQuery>>().await {
            Ok(Query(query)) => {
                info!(
                    from_date = %query.from_date,
                    to_date = %query.to_date,
                    token_address = %query.token_address,
                    "Extracted HistoricalVolatilityQuery successfully."
                );
                Ok(query)
            }
            Err(err) => Err(ApiError::InvalidQuery(err.body_text())),
        }
    }
}
