use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use crate::client::Inner;
use crate::error::Result;

/// The `automation_runs` resource (one contact's journey through one
/// automation). Reached through
/// [`Mailtea::automation_runs`](crate::Mailtea::automation_runs).
///
/// Runs are nested under an automation and scoped to a publication — pass
/// `automation_id` and `publication_id`. A run PINS the automation version it
/// started on, so [`get`](AutomationRuns::get) returns the graph the run is
/// actually executing, not the live one.
#[derive(Clone, Debug)]
pub struct AutomationRuns {
    inner: Arc<Inner>,
}

impl AutomationRuns {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// `GET /v1/automations/:automation_id/runs` — list runs, cursor-paginated.
    ///
    /// Filters: `publication_id` (required), `status` (one or more run
    /// statuses — pass an array and it is comma-joined for you), `contact_id`,
    /// `is_test`, `limit`, `after`. List items omit the pinned graph and the
    /// step runs; use [`get`](Self::get) for those.
    pub async fn list(&self, automation_id: &str, params: impl Serialize) -> Result<Value> {
        let mut payload = crate::params::to_value(params)?;
        // The server matches `is_test` against the literal strings
        // "true"/"false" and 400s on anything else, so a JSON boolean cannot go
        // out as-is.
        if let Some(map) = payload.as_object_mut() {
            if let Some(Value::Bool(flag)) = map.get("is_test").cloned() {
                map.insert("is_test".to_string(), Value::String(flag.to_string()));
            }
        }
        self.inner
            .call(
                "GET",
                &format!(
                    "/v1/automations/{}/runs{}",
                    crate::params::encode(automation_id),
                    crate::params::query_of_value(&payload)
                ),
                None,
            )
            .await
    }

    /// `GET /v1/automations/:automation_id/runs/:run_id` — one run in full.
    /// Requires `publication_id`.
    ///
    /// Returns the PINNED `steps`/`connections`, the per-step `step_runs`, and
    /// `waiting` (`resume_at` / `waiting_event_name`) — read this rather than an
    /// event ingest's `resumed_runs` counter to tell whether an event actually
    /// advanced the run.
    pub async fn get(
        &self,
        automation_id: &str,
        run_id: &str,
        params: impl Serialize,
    ) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!(
                    "/v1/automations/{}/runs/{}{}",
                    crate::params::encode(automation_id),
                    crate::params::encode(run_id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `POST /v1/automations/:automation_id/runs/:run_id/cancel` — cancel one
    /// in-flight run. Requires `publication_id`. A cancelled run cannot be
    /// resumed.
    pub async fn cancel(
        &self,
        automation_id: &str,
        run_id: &str,
        params: impl Serialize,
    ) -> Result<Value> {
        self.inner
            .call(
                "POST",
                &format!(
                    "/v1/automations/{}/runs/{}/cancel{}",
                    crate::params::encode(automation_id),
                    crate::params::encode(run_id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }
}
