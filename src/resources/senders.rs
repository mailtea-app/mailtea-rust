use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use crate::client::Inner;
use crate::error::Result;
use crate::params;

/// The `senders` resource (named From identities). Reached through
/// [`Mailtea::senders`](crate::Mailtea::senders).
///
/// Senders are scoped to a publication — pass `publication_id`.
/// [`create`](Senders::create) takes `name` and `email` (the address must live
/// on a verified, DKIM-verified email domain), plus optional `reply_to` and
/// `is_default`. The `email` is immutable, so [`update`](Senders::update) only
/// changes `name`, `reply_to` and `is_default`.
#[derive(Clone, Debug)]
pub struct Senders {
    inner: Arc<Inner>,
}

impl Senders {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// `POST /v1/senders` — create a sender.
    pub async fn create(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call("POST", "/v1/senders", crate::params::to_body(params)?)
            .await
    }

    /// `GET /v1/senders` — list senders, cursor-paginated. Filters:
    /// `publication_id` (required), `limit`, `after`.
    pub async fn list(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!("/v1/senders{}", params::query(params)?),
                None,
            )
            .await
    }

    /// `GET /v1/senders/:id` — one sender. Requires `publication_id`.
    pub async fn get(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!(
                    "/v1/senders/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `PATCH /v1/senders/:id` — update `name`, `reply_to` or `is_default`.
    /// `publication_id` is required, in the body.
    pub async fn update(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "PATCH",
                &format!("/v1/senders/{}", crate::params::encode(id)),
                crate::params::to_body(params)?,
            )
            .await
    }

    /// `DELETE /v1/senders/:id` — delete a sender. Requires `publication_id`.
    pub async fn delete(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "DELETE",
                &format!(
                    "/v1/senders/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }
}
