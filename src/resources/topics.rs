use std::sync::Arc;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::client::Inner;
use crate::error::Result;
use crate::params;

/// The `topics` resource (topic definitions). Reached through
/// [`Mailtea::topics`](crate::Mailtea::topics).
///
/// Topics are scoped to a publication — pass `publication_id`. This manages
/// topic definitions only; assigning topics to contacts is not exposed yet.
#[derive(Clone, Debug)]
pub struct Topics {
    inner: Arc<Inner>,
}

impl Topics {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// `POST /v1/topics` — create a topic definition. Requires
    /// `publication_id`, `name` and `default_subscription` (`opt_in` or
    /// `opt_out`). Optional `description` and `visibility` (`private` by
    /// default; `public` makes the topic its own subscription on the reader
    /// preference page).
    pub async fn create(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call("POST", "/v1/topics", crate::params::to_body(params)?)
            .await
    }

    /// `GET /v1/topics` — list topics. Requires `publication_id`.
    pub async fn list(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!("/v1/topics{}", params::query(params)?),
                None,
            )
            .await
    }

    /// `GET /v1/topics/:id` — one topic. Requires `publication_id`.
    pub async fn get(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!(
                    "/v1/topics/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `PATCH /v1/topics/:id` — update a topic. `publication_id` rides in both
    /// the query and the body.
    pub async fn update(&self, id: &str, params: impl Serialize) -> Result<Value> {
        let payload = crate::params::to_value(params)?;
        self.inner
            .call(
                "PATCH",
                &format!(
                    "/v1/topics/{}{}",
                    crate::params::encode(id),
                    crate::params::query_pick(&payload, &["publication_id"])
                ),
                crate::params::body_from_value(&payload)?,
            )
            .await
    }

    /// `DELETE /v1/topics/:id` — delete a topic. Requires `publication_id`.
    pub async fn delete(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "DELETE",
                &format!(
                    "/v1/topics/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }
}

/// The payload for [`Topics::create`].
#[derive(Clone, Debug, Default, Serialize)]
pub struct CreateTopic {
    pub publication_id: String,
    pub name: String,
    /// `opt_in` or `opt_out` — required, and the field most easily forgotten.
    pub default_subscription: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// `private` (the default) or `public`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl CreateTopic {
    pub fn new(
        publication_id: impl Into<String>,
        name: impl Into<String>,
        default_subscription: impl Into<String>,
    ) -> Self {
        Self {
            publication_id: publication_id.into(),
            name: name.into(),
            default_subscription: default_subscription.into(),
            description: None,
            visibility: None,
            extra: Map::new(),
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn visibility(mut self, visibility: impl Into<String>) -> Self {
        self.visibility = Some(visibility.into());
        self
    }
}
