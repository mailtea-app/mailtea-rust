use std::sync::Arc;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::client::Inner;
use crate::error::Result;
use crate::params;

/// The `contacts` resource. Reached through
/// [`Mailtea::contacts`](crate::Mailtea::contacts).
///
/// Audience resources are scoped to a publication — pass `publication_id`.
#[derive(Clone, Debug)]
pub struct Contacts {
    inner: Arc<Inner>,
}

impl Contacts {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// `POST /v1/contacts` — create a contact, or update it if the email
    /// already exists in the publication (the endpoint upserts).
    /// [`upsert`](Self::upsert) is the same call under the name of what it does.
    pub async fn create(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call("POST", "/v1/contacts", crate::params::to_body(params)?)
            .await
    }

    /// Create the contact or update it in place — an alias of
    /// [`create`](Self::create), named for what `POST /v1/contacts` does.
    pub async fn upsert(&self, params: impl Serialize) -> Result<Value> {
        self.create(params).await
    }

    /// `GET /v1/contacts` — list contacts, cursor-paginated. Filters:
    /// `publication_id` (required), `status`
    /// (`active`/`unsubscribed`/`suppressed`), `search` (matches the email
    /// address), `limit`, `after` (cursor from a previous `next_cursor`).
    pub async fn list(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!("/v1/contacts{}", params::query(params)?),
                None,
            )
            .await
    }

    /// `GET /v1/contacts/:id_or_email` — one contact, by id or email address.
    pub async fn get(&self, id_or_email: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!(
                    "/v1/contacts/{}{}",
                    crate::params::encode(id_or_email),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `PATCH /v1/contacts/:id_or_email` — update a contact. `publication_id`
    /// rides in both the query and the body, which is what the endpoint expects.
    pub async fn update(&self, id_or_email: &str, params: impl Serialize) -> Result<Value> {
        let payload = crate::params::to_value(params)?;
        self.inner
            .call(
                "PATCH",
                &format!(
                    "/v1/contacts/{}{}",
                    crate::params::encode(id_or_email),
                    crate::params::query_pick(&payload, &["publication_id"])
                ),
                crate::params::body_from_value(&payload)?,
            )
            .await
    }

    /// `DELETE /v1/contacts/:id_or_email` — remove a contact. Requires
    /// `publication_id`.
    pub async fn delete(&self, id_or_email: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "DELETE",
                &format!(
                    "/v1/contacts/{}{}",
                    crate::params::encode(id_or_email),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }
}

/// The payload for [`Contacts::create`] / [`Contacts::upsert`].
#[derive(Clone, Debug, Default, Serialize)]
pub struct CreateContact {
    pub publication_id: String,
    pub email: String,
    /// `active`, `unsubscribed` or `suppressed`. Omitted means `active`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Anything this struct does not name yet.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl CreateContact {
    pub fn new(publication_id: impl Into<String>, email: impl Into<String>) -> Self {
        Self {
            publication_id: publication_id.into(),
            email: email.into(),
            status: None,
            extra: Map::new(),
        }
    }

    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }
}

/// The payload for [`Contacts::update`].
#[derive(Clone, Debug, Default, Serialize)]
pub struct UpdateContact {
    pub publication_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl UpdateContact {
    pub fn new(publication_id: impl Into<String>) -> Self {
        Self {
            publication_id: publication_id.into(),
            status: None,
            extra: Map::new(),
        }
    }

    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }
}
