use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use crate::client::Inner;
use crate::error::Result;
use crate::params;

/// The `suppressions` resource (the org-wide do-not-send list). Reached through
/// [`Mailtea::suppressions`](crate::Mailtea::suppressions).
///
/// Suppressions are team-scoped — there is no `publication_id`.
#[derive(Clone, Debug)]
pub struct Suppressions {
    inner: Arc<Inner>,
}

impl Suppressions {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// `GET /v1/suppressions` — list entries, cursor-paginated. Optional
    /// filters: `reason`, `q` (email search), `created_after`,
    /// `created_before`, `limit`, `starting_after`.
    pub async fn list(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!("/v1/suppressions{}", params::query(params)?),
                None,
            )
            .await
    }

    /// `POST /v1/suppressions` — add addresses. Takes `emails` (up to 1000) and
    /// an optional `reason`. Returns `{"added": ...}`.
    pub async fn add(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call("POST", "/v1/suppressions", crate::params::to_body(params)?)
            .await
    }

    /// `DELETE /v1/suppressions` — remove addresses. Takes `emails`. Returns
    /// `{"removed": ...}`.
    pub async fn remove(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "DELETE",
                "/v1/suppressions",
                crate::params::to_body(params)?,
            )
            .await
    }

    /// `GET /v1/suppressions/export` — the whole list as CSV.
    ///
    /// Returns the raw `text/csv` body (`email,reason,source,created_at` with a
    /// header row), not JSON.
    pub async fn export(&self) -> Result<String> {
        self.inner
            .call_text("GET", "/v1/suppressions/export", None)
            .await
    }
}
