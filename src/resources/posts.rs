use std::sync::Arc;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::client::Inner;
use crate::error::Result;
use crate::params;

/// The `posts` resource (newsletter posts and broadcasts). Reached through
/// [`Mailtea::posts`](crate::Mailtea::posts).
#[derive(Clone, Debug)]
pub struct Posts {
    inner: Arc<Inner>,
}

impl Posts {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// `POST /v1/posts` — create a post (a draft by default).
    ///
    /// Seed it from a published server template with `template_id` +
    /// `variables`, or pass inline `html`. `kind` selects the post type
    /// (`newsletter` or `broadcast`). Set `send: true` to deliver right after
    /// creating (or with `scheduled_at` to schedule) — that requires the
    /// `issues:send` scope.
    pub async fn create(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call("POST", "/v1/posts", crate::params::to_body(params)?)
            .await
    }

    /// `GET /v1/posts` — list posts, most recent first, offset-paginated.
    /// Takes `publication_id` (required) plus optional `limit`, `offset`,
    /// `status` and `kind`.
    pub async fn list(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call("GET", &format!("/v1/posts{}", params::query(params)?), None)
            .await
    }

    /// `GET /v1/posts/:id` — one post.
    pub async fn get(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!(
                    "/v1/posts/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `PATCH /v1/posts/:id` — update a draft post (sent posts are immutable).
    /// Accepts `subject`, `html`, `text`, `from`, `reply_to` and `name`.
    pub async fn update(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "PATCH",
                &format!("/v1/posts/{}", crate::params::encode(id)),
                crate::params::to_body(params)?,
            )
            .await
    }

    /// `DELETE /v1/posts/:id` — delete a draft post (sent posts cannot be
    /// deleted).
    pub async fn delete(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "DELETE",
                &format!(
                    "/v1/posts/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `POST /v1/posts/:id/send` — send a draft post to the publication's
    /// audience, now or at `scheduled_at` (RFC 3339). Requires the
    /// `issues:send` scope. Pass `()` to send immediately.
    pub async fn send(&self, id: &str, params: impl Serialize) -> Result<Value> {
        // An empty object is sent as no body at all: the endpoint's schema
        // takes an optional `scheduled_at` and nothing else.
        let payload = crate::params::to_value(params)?;
        self.inner
            .call(
                "POST",
                &format!("/v1/posts/{}/send", crate::params::encode(id)),
                crate::params::body_or_none(&payload)?,
            )
            .await
    }

    /// `POST /v1/posts/:id/test` — send a TEST copy of a post to specific
    /// recipients before subscribers see it.
    ///
    /// It renders the post exactly as a subscriber would receive it and
    /// delivers a one-shot `[TEST]` email — it does NOT send to the audience.
    /// Takes `recipients` (up to 10), `from` (must use a verified domain), and
    /// optional `reply_to`.
    pub async fn send_test(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "POST",
                &format!("/v1/posts/{}/test", crate::params::encode(id)),
                crate::params::to_body(params)?,
            )
            .await
    }
}

/// The payload for [`Posts::create`].
#[derive(Clone, Debug, Default, Serialize)]
pub struct CreatePost {
    pub publication_id: String,
    pub subject: String,
    /// Inline HTML. Mutually exclusive with `template_id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Seed the post from a published server template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub variables: Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    /// The internal name, when it should differ from the subject.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `newsletter` or `broadcast`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Deliver right after creating. Requires the `issues:send` scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send: Option<bool>,
    /// With `send`, schedules instead of sending now. RFC 3339.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl CreatePost {
    pub fn new(publication_id: impl Into<String>, subject: impl Into<String>) -> Self {
        Self {
            publication_id: publication_id.into(),
            subject: subject.into(),
            ..Default::default()
        }
    }

    pub fn html(mut self, html: impl Into<String>) -> Self {
        self.html = Some(html.into());
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn template_id(mut self, template_id: impl Into<String>) -> Self {
        self.template_id = Some(template_id.into());
        self
    }

    pub fn variable(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }

    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    pub fn reply_to(mut self, reply_to: impl Into<String>) -> Self {
        self.reply_to = Some(reply_to.into());
        self
    }

    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    /// Deliver on create, optionally at `scheduled_at`.
    pub fn send(mut self, send: bool) -> Self {
        self.send = Some(send);
        self
    }

    pub fn scheduled_at(mut self, scheduled_at: impl Into<String>) -> Self {
        self.scheduled_at = Some(scheduled_at.into());
        self
    }
}

/// The payload for [`Posts::send`]: schedule it, or send now with `()`.
#[derive(Clone, Debug, Default, Serialize)]
pub struct SendPost {
    /// RFC 3339. Omit to send immediately.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<String>,
}

impl SendPost {
    /// Schedule the send for an RFC 3339 instant.
    pub fn at(scheduled_at: impl Into<String>) -> Self {
        Self {
            scheduled_at: Some(scheduled_at.into()),
        }
    }
}

/// The payload for [`Posts::send_test`].
#[derive(Clone, Debug, Default, Serialize)]
pub struct SendTestPost {
    /// Up to 10 addresses.
    pub recipients: Vec<String>,
    /// Must use a verified email-purpose domain.
    pub from: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl SendTestPost {
    pub fn new(
        recipients: impl IntoIterator<Item = impl Into<String>>,
        from: impl Into<String>,
    ) -> Self {
        Self {
            recipients: recipients.into_iter().map(Into::into).collect(),
            from: from.into(),
            reply_to: None,
            extra: Map::new(),
        }
    }

    pub fn reply_to(mut self, reply_to: impl Into<String>) -> Self {
        self.reply_to = Some(reply_to.into());
        self
    }
}
