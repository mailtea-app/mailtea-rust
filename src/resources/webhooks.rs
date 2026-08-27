use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use crate::client::Inner;
use crate::error::Result;
use crate::params;

const BASE: &str = "/v1/webhooks/endpoints";

/// The `webhooks` resource (outbound event subscriptions). Reached through
/// [`Mailtea::webhooks`](crate::Mailtea::webhooks).
///
/// Scoped to a publication — pass `publication_id`.
/// [`create`](Webhooks::create) returns the `signing_secret` **once**; store it
/// and hand it to
/// [`verify_webhook_signature`](crate::verify_webhook_signature).
#[derive(Clone, Debug)]
pub struct Webhooks {
    inner: Arc<Inner>,
}

impl Webhooks {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// `POST /v1/webhooks/endpoints` — subscribe an endpoint to events.
    pub async fn create(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call("POST", BASE, crate::params::to_body(params)?)
            .await
    }

    /// `GET /v1/webhooks/endpoints` — list endpoints. Requires
    /// `publication_id`.
    pub async fn list(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call("GET", &format!("{BASE}{}", params::query(params)?), None)
            .await
    }

    /// `GET /v1/webhooks/endpoints/:id` — one endpoint. Requires
    /// `publication_id`.
    pub async fn get(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!(
                    "{BASE}/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `PATCH /v1/webhooks/endpoints/:id` — update an endpoint's URL, events or
    /// enabled state. `publication_id` rides in both the query and the body.
    pub async fn update(&self, id: &str, params: impl Serialize) -> Result<Value> {
        let payload = crate::params::to_value(params)?;
        self.inner
            .call(
                "PATCH",
                &format!(
                    "{BASE}/{}{}",
                    crate::params::encode(id),
                    crate::params::query_pick(&payload, &["publication_id"])
                ),
                crate::params::body_from_value(&payload)?,
            )
            .await
    }

    /// `DELETE /v1/webhooks/endpoints/:id` — unsubscribe. Requires
    /// `publication_id`.
    pub async fn delete(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "DELETE",
                &format!(
                    "{BASE}/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }
}
