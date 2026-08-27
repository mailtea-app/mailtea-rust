use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use crate::client::Inner;
use crate::error::Result;
use crate::params;

/// The `events` resource (custom product events that trigger automations and
/// resume `wait_for_event` steps). Reached through
/// [`Mailtea::events`](crate::Mailtea::events).
///
/// Events are scoped to a publication — pass `publication_id`.
#[derive(Clone, Debug)]
pub struct Events {
    inner: Arc<Inner>,
}

impl Events {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// `POST /v1/events` — record an event for a contact.
    ///
    /// Takes `publication_id`, `name`, and exactly one of `contact_id` or
    /// `email` (both is a 400 `contact_reference_conflict`, neither a 400
    /// `contact_reference_required`). Optional: `create_contact`, `properties`,
    /// `occurred_at`, `idempotency_key`.
    ///
    /// `create_contact` is **opt-in** — without it an unresolvable address is a
    /// 404 `contact_not_found` rather than a new contact.
    ///
    /// Returns 202 with `enrolled_automations` and `resumed_runs`. A replay of
    /// the same `idempotency_key` returns the ORIGINAL event id with
    /// `replayed: true` and always reports zeros. `resumed_runs: 0` on a FRESH
    /// ingest does not prove nothing matched — a run being advanced
    /// concurrently is invisible for that instant, so read the run itself with
    /// [`AutomationRuns::get`](crate::AutomationRuns::get) rather than the
    /// counter.
    pub async fn send(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call("POST", "/v1/events", crate::params::to_body(params)?)
            .await
    }

    /// `GET /v1/events` — list recorded events, cursor-paginated. Filters:
    /// `publication_id` (required), `name`, `contact_id`, `limit`, `after`.
    pub async fn list(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!("/v1/events{}", params::query(params)?),
                None,
            )
            .await
    }
}

/// The `event_definitions` resource (the catalog of event names a publication
/// expects, with optional property schemas). Reached through
/// [`Mailtea::event_definitions`](crate::Mailtea::event_definitions).
///
/// Definitions are scoped to a publication — pass `publication_id`. They are
/// documentation and tooling, not a gate: [`Events::send`] accepts an event with
/// no definition.
#[derive(Clone, Debug)]
pub struct EventDefinitions {
    inner: Arc<Inner>,
}

impl EventDefinitions {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// `POST /v1/event-definitions` — create a definition. Takes
    /// `publication_id` and `name`, plus optional `description` and
    /// `schema_json`. The name is immutable once created.
    pub async fn create(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "POST",
                "/v1/event-definitions",
                crate::params::to_body(params)?,
            )
            .await
    }

    /// `GET /v1/event-definitions` — list definitions, cursor-paginated.
    /// Filters: `publication_id` (required), `limit`, `after`. List items carry
    /// no `inferred_properties` — use [`get`](Self::get) for those.
    pub async fn list(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!("/v1/event-definitions{}", params::query(params)?),
                None,
            )
            .await
    }

    /// `GET /v1/event-definitions/:id` — one definition. Requires
    /// `publication_id`.
    ///
    /// Adds `schema_properties` and `inferred_properties` — the latter computed
    /// on read over the last 500 events, reporting each key's type, sample count
    /// and **coverage**. Low coverage is the trap: a condition on a key present
    /// in 3% of events will almost never match.
    pub async fn get(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!(
                    "/v1/event-definitions/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `PATCH /v1/event-definitions/:id` — update `description` or
    /// `schema_json` (`null` clears the schema back to free-form).
    /// `publication_id` is required and is sent as a query parameter — it is
    /// removed from the body, because `name` is immutable and the endpoint
    /// rejects unexpected fields.
    pub async fn update(&self, id: &str, params: impl Serialize) -> Result<Value> {
        let mut payload = crate::params::to_value(params)?;
        let publication_id = crate::params::take(&mut payload, "publication_id");
        self.inner
            .call(
                "PATCH",
                &format!(
                    "/v1/event-definitions/{}{}",
                    crate::params::encode(id),
                    crate::params::query_of_value(
                        &serde_json::json!({ "publication_id": publication_id })
                    )
                ),
                crate::params::body_from_value(&payload)?,
            )
            .await
    }

    /// `DELETE /v1/event-definitions/:id` — delete a definition. Requires
    /// `publication_id`. Events already recorded under that name are untouched.
    pub async fn delete(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "DELETE",
                &format!(
                    "/v1/event-definitions/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }
}
