use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use crate::client::Inner;
use crate::error::Result;
use crate::params;

/// The `segments` resource. Reached through
/// [`Mailtea::segments`](crate::Mailtea::segments).
///
/// Audience segments are scoped to a publication — pass `publication_id`. To
/// clear a nullable filter on update, send it as `null`
/// (`json!({"status_filter": null})`); leave the key out to keep it unchanged.
#[derive(Clone, Debug)]
pub struct Segments {
    inner: Arc<Inner>,
}

impl Segments {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// `POST /v1/segments` — create a segment.
    pub async fn create(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call("POST", "/v1/segments", crate::params::to_body(params)?)
            .await
    }

    /// `GET /v1/segments` — list segments. Requires `publication_id`.
    pub async fn list(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!("/v1/segments{}", params::query(params)?),
                None,
            )
            .await
    }

    /// `GET /v1/segments/:id` — one segment. Requires `publication_id`.
    pub async fn get(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!(
                    "/v1/segments/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `PATCH /v1/segments/:id` — update a segment. `publication_id` rides in
    /// both the query and the body.
    pub async fn update(&self, id: &str, params: impl Serialize) -> Result<Value> {
        let payload = crate::params::to_value(params)?;
        self.inner
            .call(
                "PATCH",
                &format!(
                    "/v1/segments/{}{}",
                    crate::params::encode(id),
                    crate::params::query_pick(&payload, &["publication_id"])
                ),
                crate::params::body_from_value(&payload)?,
            )
            .await
    }

    /// `DELETE /v1/segments/:id` — delete a segment. Requires `publication_id`.
    pub async fn delete(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "DELETE",
                &format!(
                    "/v1/segments/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }
}
