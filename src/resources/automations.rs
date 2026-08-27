use std::sync::Arc;

use serde::Serialize;
use serde_json::{json, Value};

use crate::client::Inner;
use crate::error::Result;
use crate::params;

/// The `automations` resource (multi-step contact journeys). Reached through
/// [`Mailtea::automations`](crate::Mailtea::automations).
///
/// Automations are scoped to a publication — pass `publication_id`. An
/// automation is a graph: `steps` (each `{"key", "type", "label", "config"}`)
/// plus optional `connections` (each `{"from", "to", "branch"}`).
///
/// `connections` is optional: omit it and the server links the steps in array
/// order with `branch: "next"`, rooted at the trigger. A graph containing a
/// `condition` or `wait_for_event` step cannot be inferred that way and is
/// rejected with `connections_required_for_branching` — send its connections
/// explicitly.
///
/// Failures come back as coded `issues[]` rather than schema errors, and for a
/// draft/paused/archived automation they ride along informationally instead of
/// blocking the save.
#[derive(Clone, Debug)]
pub struct Automations {
    inner: Arc<Inner>,
}

impl Automations {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// `POST /v1/automations/validate` — dry-run a graph without creating
    /// anything. Takes `publication_id` and `steps`, plus optional
    /// `connections`.
    pub async fn validate(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "POST",
                "/v1/automations/validate",
                crate::params::to_body(params)?,
            )
            .await
    }

    /// `POST /v1/automations` — create an automation.
    ///
    /// Takes `publication_id`, `name` and `steps`, plus optional `description`,
    /// `connections`, `reentry_policy`
    /// (`once`/`once_per_window`/`always` — `once_per_window` requires
    /// `reentry_window_seconds`), `on_step_failure` and `validate_only`. With
    /// `validate_only: true` nothing is written and an `automation_validation`
    /// is returned instead. New automations start as `draft` —
    /// [`activate`](Self::activate) starts them.
    pub async fn create(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call("POST", "/v1/automations", crate::params::to_body(params)?)
            .await
    }

    /// `GET /v1/automations` — list automations, cursor-paginated. Filters:
    /// `publication_id` (required), `status`, `limit`, `after`. List items omit
    /// `steps`, `connections`, `valid` and `issues` — use [`get`](Self::get) for
    /// the full graph.
    pub async fn list(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!("/v1/automations{}", params::query(params)?),
                None,
            )
            .await
    }

    /// `GET /v1/automations/:id` — one automation with its live graph and
    /// current `issues[]`. Requires `publication_id`.
    pub async fn get(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!(
                    "/v1/automations/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `PATCH /v1/automations/:id` — replace the graph or its settings.
    ///
    /// `publication_id` is required and is sent as a query parameter — it is
    /// removed from the body, which the endpoint's schema does not accept it in.
    /// The graph is replaced wholesale and cuts a new version.
    ///
    /// `validate_only: true` returns an `automation_validation` and writes
    /// nothing. A graph change that carries errors saves anyway while the
    /// automation is draft/paused/archived; on an `active` one it is a 422 —
    /// pause, save, then start again.
    pub async fn update(&self, id: &str, params: impl Serialize) -> Result<Value> {
        let mut payload = crate::params::to_value(params)?;
        let publication_id = crate::params::take(&mut payload, "publication_id");
        self.inner
            .call(
                "PATCH",
                &format!(
                    "/v1/automations/{}{}",
                    crate::params::encode(id),
                    crate::params::query_of_value(&json!({ "publication_id": publication_id }))
                ),
                crate::params::body_from_value(&payload)?,
            )
            .await
    }

    /// `DELETE /v1/automations/:id` — delete an automation. Requires
    /// `publication_id`.
    ///
    /// Deleting an `active` automation is a 409 `automation_active` — pause or
    /// archive it first so its in-flight runs are not dropped silently.
    pub async fn delete(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "DELETE",
                &format!(
                    "/v1/automations/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `POST /v1/automations/:id/activate` — start the automation so new
    /// contacts enroll. Requires `publication_id`. A graph with errors is
    /// refused with 422 `automation_invalid` and the blocking `issues[]`.
    pub async fn activate(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "POST",
                &format!(
                    "/v1/automations/{}/activate{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `POST /v1/automations/:id/pause` — stop new enrollments. Requires
    /// `publication_id` (query). Optional `cancel_runs` — it **defaults to
    /// false** here, so in-flight runs keep going; pass `cancel_runs: true` to
    /// exit them.
    pub async fn pause(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.lifecycle(id, "pause", params).await
    }

    /// `POST /v1/automations/:id/archive` — archive the automation. Requires
    /// `publication_id` (query). Optional `cancel_runs` — it **defaults to
    /// true** here (the opposite of [`pause`](Self::pause)), so in-flight runs
    /// exit with `automation_archived`.
    pub async fn archive(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.lifecycle(id, "archive", params).await
    }

    /// `GET /v1/automations/:id/versions` — list versions, cursor-paginated.
    /// Filters: `publication_id` (required), `limit`, `after`. List items carry
    /// no `steps`/`connections` — use [`version`](Self::version) for a stored
    /// graph.
    pub async fn versions(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!(
                    "/v1/automations/{}/versions{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `GET /v1/automations/:id/versions/:version` — one stored version,
    /// including its `steps` and `connections`. Requires `publication_id`.
    ///
    /// This is the graph a run of that version is pinned to — editing the
    /// automation never rewrites it.
    pub async fn version(
        &self,
        id: &str,
        version: impl std::fmt::Display,
        params: impl Serialize,
    ) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!(
                    "/v1/automations/{}/versions/{}{}",
                    crate::params::encode(id),
                    crate::params::encode(&version.to_string()),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `GET /v1/automations/:id/metrics` — per-step funnel counts. Filters:
    /// `publication_id` (required), `version` (omit to aggregate across ALL
    /// versions), `since`, `until`.
    ///
    /// Test runs are always excluded (`excludes_test_runs: true`). Condition
    /// steps report `branches: {condition_met, condition_not_met}`,
    /// `wait_for_event` steps `{event_received, timeout}`.
    pub async fn metrics(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!(
                    "/v1/automations/{}/metrics{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `POST /v1/automations/:id/test` — run the automation once against a real
    /// contact.
    ///
    /// `publication_id` is required and is sent as a query parameter; the body
    /// takes one of `contact_id` or `email`, plus optional `event_properties` to
    /// seed the run's `event.*` namespace.
    ///
    /// A test run **sends real, billed email** to that inbox — it does not
    /// bypass any send gate. It is flagged `is_test` and excluded from
    /// [`metrics`](Self::metrics). Returns 202 with the queued run.
    pub async fn test(&self, id: &str, params: impl Serialize) -> Result<Value> {
        let mut payload = crate::params::to_value(params)?;
        let publication_id = crate::params::take(&mut payload, "publication_id");
        self.inner
            .call(
                "POST",
                &format!(
                    "/v1/automations/{}/test{}",
                    crate::params::encode(id),
                    crate::params::query_of_value(&json!({ "publication_id": publication_id }))
                ),
                crate::params::body_from_value(&payload)?,
            )
            .await
    }

    /// `publication_id` goes in the query but `cancel_runs` is read from the
    /// body, so the two are split here. No body is sent when the caller omitted
    /// `cancel_runs` — or passed it as `null`, which the server's schema would
    /// reject — so the per-verb default applies in both cases.
    async fn lifecycle(&self, id: &str, verb: &str, params: impl Serialize) -> Result<Value> {
        let payload = crate::params::to_value(params)?;
        let cancel_runs = payload.get("cancel_runs").cloned().unwrap_or(Value::Null);
        let body = if cancel_runs.is_null() {
            None
        } else {
            crate::params::body_from_value(&json!({ "cancel_runs": cancel_runs }))?
        };
        self.inner
            .call(
                "POST",
                &format!(
                    "/v1/automations/{}/{verb}{}",
                    crate::params::encode(id),
                    crate::params::query_pick(&payload, &["publication_id"])
                ),
                body,
            )
            .await
    }
}
