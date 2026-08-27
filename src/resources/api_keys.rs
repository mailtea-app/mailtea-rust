use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use crate::client::Inner;
use crate::error::Result;

/// The `api_keys` resource. Reached through
/// [`Mailtea::api_keys`](crate::Mailtea::api_keys).
///
/// Requires a token with `settings:write`. A key can never be granted scopes
/// the calling token does not already hold.
#[derive(Clone, Debug)]
pub struct ApiKeys {
    inner: Arc<Inner>,
}

impl ApiKeys {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// `POST /v1/api-keys` — create an API key. The `token` is returned ONCE —
    /// store it securely.
    ///
    /// Takes `name`, optional `permission` (`"full_access"` or
    /// `"sending_access"`), and optional `domain_id`.
    pub async fn create(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call("POST", "/v1/api-keys", crate::params::to_body(params)?)
            .await
    }

    /// `GET /v1/api-keys` — list API keys (token values are never returned).
    pub async fn list(&self) -> Result<Value> {
        self.inner.call("GET", "/v1/api-keys", None).await
    }

    /// `DELETE /v1/api-keys/:id` — revoke a key.
    pub async fn revoke(&self, id: &str) -> Result<Value> {
        self.inner
            .call(
                "DELETE",
                &format!("/v1/api-keys/{}", crate::params::encode(id)),
                None,
            )
            .await
    }
}
