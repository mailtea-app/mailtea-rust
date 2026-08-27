use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use crate::client::Inner;
use crate::error::Result;
use crate::params;

/// The `contact_properties` resource (custom contact fields). Reached through
/// [`Mailtea::contact_properties`](crate::Mailtea::contact_properties).
///
/// Definitions are team-scoped — there is no `publication_id`.
/// [`create`](ContactProperties::create) takes `key` and `type` (`"string"` or
/// `"number"`).
#[derive(Clone, Debug)]
pub struct ContactProperties {
    inner: Arc<Inner>,
}

impl ContactProperties {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// `POST /v1/contact-properties` — define a custom field.
    pub async fn create(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "POST",
                "/v1/contact-properties",
                crate::params::to_body(params)?,
            )
            .await
    }

    /// `GET /v1/contact-properties` — list the definitions.
    pub async fn list(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!("/v1/contact-properties{}", params::query(params)?),
                None,
            )
            .await
    }

    /// `PATCH /v1/contact-properties/:id` — rename or retype a definition.
    pub async fn update(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "PATCH",
                &format!("/v1/contact-properties/{}", crate::params::encode(id)),
                crate::params::to_body(params)?,
            )
            .await
    }

    /// `DELETE /v1/contact-properties/:id` — delete a definition.
    pub async fn delete(&self, id: &str) -> Result<Value> {
        self.inner
            .call(
                "DELETE",
                &format!("/v1/contact-properties/{}", crate::params::encode(id)),
                None,
            )
            .await
    }
}
