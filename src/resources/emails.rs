use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::client::Inner;
use crate::error::Result;
use crate::params;

use super::inbound::InboundEmails;

/// The `emails` resource. Reached through [`Mailtea::emails`](crate::Mailtea::emails).
///
/// Every method takes `impl Serialize`, so a payload may be a typed
/// [`SendEmail`], a `serde_json::json!({...})` literal, or any struct of your
/// own that serializes to the wire shape.
#[derive(Clone, Debug)]
pub struct Emails {
    inner: Arc<Inner>,
    /// Inbound (received) emails: list, get, reply, and attachments.
    pub inbound: InboundEmails,
}

impl Emails {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self {
            inbound: InboundEmails::new(Arc::clone(&inner)),
            inner,
        }
    }

    /// `POST /v1/emails` — send a transactional email. Provide `html`/`text` OR
    /// a `template`.
    ///
    /// Set the From with exactly one of `from` (a `Name <email>` string) or
    /// `sender_id` (the id of a named, verified publication sender, which also
    /// supplies its default `reply_to`).
    ///
    /// `to`, `cc` and `bcc` are capped at **50 recipients combined** — the
    /// provider refuses more, so accepting them would answer 200 and die
    /// downstream where you never see it. Add `scheduled_at` (RFC 3339) to
    /// schedule instead of sending now.
    pub async fn send(&self, email: impl Serialize) -> Result<SentEmail> {
        self.send_idempotent(email, None).await
    }

    /// The same send, tagged with a key of your choosing.
    ///
    /// This is what makes "back off and try again" safe. A timeout or a 5xx does
    /// not tell you whether the message went out — the API may have accepted it
    /// and the answer got lost on the way back — so a bare retry can deliver it
    /// twice. Replaying the SAME key with the SAME body returns the original
    /// result instead of sending again; the same key with a different body is
    /// refused with a 409.
    ///
    /// Use an id your own system already has and will reproduce on the retry:
    /// the order, the job, the row you are notifying about. A fresh random key
    /// per attempt protects nothing.
    pub async fn send_idempotent(
        &self,
        email: impl Serialize,
        idempotency_key: Option<&str>,
    ) -> Result<SentEmail> {
        // A header, not a body field — the schema would reject it as one.
        let headers: Vec<(String, String)> = idempotency_key
            .map(|key| vec![("Idempotency-Key".to_string(), key.to_string())])
            .unwrap_or_default();
        self.inner
            .call_with_headers("POST", "/v1/emails", params::to_body(email)?, &headers)
            .await
    }

    /// `POST /v1/emails/batch` — send up to 100 emails in one request.
    pub async fn batch(&self, emails: impl Serialize) -> Result<BatchSent> {
        self.inner
            .call("POST", "/v1/emails/batch", params::to_body(emails)?)
            .await
    }

    /// `GET /v1/emails/:id` — an email with its delivery status and tracking
    /// counters.
    ///
    /// [`Email::status`] is filled in from the wire's `last_event` when the API
    /// does not send one, so the friendly name always reads.
    pub async fn get(&self, id: &str) -> Result<Email> {
        let mut email: Email = self
            .inner
            .call("GET", &format!("/v1/emails/{}", params::encode(id)), None)
            .await?;
        if email.status.is_none() {
            email.status = email.last_event.clone();
        }
        Ok(email)
    }

    /// `GET /v1/emails` — list emails, most recent first.
    ///
    /// Optional filters: `status`, `tag_name`, `tag_value`, `search` (substring
    /// match on recipient/sender/subject), `from_date`, `to_date`, `limit`,
    /// `offset`. Pass `()` for none.
    ///
    /// `from_date` is clamped to the plan's analytics retention window — 30 days
    /// on most plans, 90 on Scale and Enterprise. A value reaching further back
    /// returns data from the start of that window rather than an error, and
    /// omitting it returns the window rather than all time.
    pub async fn list(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!("/v1/emails{}", params::query(params)?),
                None,
            )
            .await
    }

    /// `GET /v1/emails/analytics` — aggregate transactional metrics over an
    /// optional date window: totals, delivered/bounced/open/click counts,
    /// per-status counts, and rates. Optional `from_date`, `to_date`.
    ///
    /// `from_date` is clamped to the plan's retention window the same way
    /// [`list`](Self::list) describes; the response reports the window used.
    pub async fn analytics(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!("/v1/emails/analytics{}", params::query(params)?),
                None,
            )
            .await
    }

    /// `PATCH /v1/emails/:id` — update a scheduled email (currently only
    /// `scheduled_at`).
    pub async fn update(&self, id: &str, params: impl Serialize) -> Result<Email> {
        self.inner
            .call(
                "PATCH",
                &format!("/v1/emails/{}", crate::params::encode(id)),
                crate::params::to_body(params)?,
            )
            .await
    }

    /// [`update`](Self::update) for the reschedule case.
    pub async fn reschedule(&self, id: &str, scheduled_at: &str) -> Result<Email> {
        self.update(id, serde_json::json!({ "scheduled_at": scheduled_at }))
            .await
    }

    /// `POST /v1/emails/:id/cancel` — cancel a scheduled email before it sends.
    ///
    /// There is no `DELETE` on emails. Cancelling works only while the send is
    /// still `scheduled`; after that the API answers 422 and the message is
    /// already on its way.
    pub async fn cancel(&self, id: &str) -> Result<Email> {
        self.inner
            .call(
                "POST",
                &format!("/v1/emails/{}/cancel", params::encode(id)),
                None,
            )
            .await
    }
}

/// A message to send.
///
/// `Default` covers the optional half, so both styles work:
///
/// ```
/// use mailtea::{SendEmail, Tag};
///
/// let chained = SendEmail::new("Acme <hello@acme.com>", ["reader@example.com"], "Hi")
///     .html("<p>Hi</p>")
///     .tag("category", "receipt");
///
/// let literal = SendEmail {
///     from: Some("Acme <hello@acme.com>".to_string()),
///     to: vec!["reader@example.com".to_string()],
///     subject: Some("Hi".to_string()),
///     tags: vec![Tag::new("category", "receipt")],
///     ..Default::default()
/// };
/// ```
///
/// Unset fields are left off the wire rather than sent as `null` — an empty `cc`
/// or a null `scheduled_at` would turn an immediate send into a rejected one.
#[derive(Clone, Debug, Default, Serialize)]
pub struct SendEmail {
    /// `Name <you@your-verified-domain.com>`, or a bare address. Exactly one of
    /// this and `sender_id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// The id of a named, verified publication sender. It supplies the From and
    /// its default `reply_to`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_id: Option<String>,
    /// At most 50 recipients across `to`, `cc` and `bcc` together.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub to: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// Send `html`, `text`, or both. Both is what inboxes prefer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Render a published server template instead of inline `html`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<TemplateRef>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cc: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bcc: Vec<String>,
    /// Not a recipient, so it does not count against the 50.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reply_to: Vec<String>,
    /// Labels carried with the send and echoed back on its events.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Tag>,
    /// Extra custom email headers.
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub headers: Map<String, Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Attachment>,
    /// RFC 3339 in UTC, e.g. `2026-09-01T09:00:00Z`. Omit to send immediately.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<String>,
    /// Opt this message out of the open pixel. A sending domain with tracking
    /// switched off cannot be overridden here — policy narrows, never widens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracking_open: Option<bool>,
    /// Opt this message out of link rewriting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracking_click: Option<bool>,
    /// Anything this struct does not name yet, merged into the same JSON object.
    /// The escape hatch for a field the API grew after this release.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl SendEmail {
    /// The usual send: a From address, recipients, and a subject.
    pub fn new(
        from: impl Into<String>,
        to: impl IntoIterator<Item = impl Into<String>>,
        subject: impl Into<String>,
    ) -> Self {
        Self {
            from: Some(from.into()),
            to: to.into_iter().map(Into::into).collect(),
            subject: Some(subject.into()),
            ..Default::default()
        }
    }

    /// The same, From a named publication sender instead of a literal address.
    pub fn from_sender(
        sender_id: impl Into<String>,
        to: impl IntoIterator<Item = impl Into<String>>,
        subject: impl Into<String>,
    ) -> Self {
        Self {
            sender_id: Some(sender_id.into()),
            to: to.into_iter().map(Into::into).collect(),
            subject: Some(subject.into()),
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

    /// Render a published server template, with `variables` substituted.
    pub fn template(mut self, template: TemplateRef) -> Self {
        self.template = Some(template);
        self
    }

    pub fn cc(mut self, cc: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.cc = cc.into_iter().map(Into::into).collect();
        self
    }

    pub fn bcc(mut self, bcc: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.bcc = bcc.into_iter().map(Into::into).collect();
        self
    }

    pub fn reply_to(mut self, reply_to: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.reply_to = reply_to.into_iter().map(Into::into).collect();
        self
    }

    /// Add one tag. Call it repeatedly for several.
    pub fn tag(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.push(Tag::new(name, value));
        self
    }

    /// Add one custom header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers
            .insert(name.into(), Value::String(value.into()));
        self
    }

    /// Add one attachment. See [`Attachment::from_bytes`] for the common case.
    pub fn attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Schedule the send. RFC 3339 in UTC, e.g. `2026-09-01T09:00:00Z`.
    pub fn scheduled_at(mut self, scheduled_at: impl Into<String>) -> Self {
        self.scheduled_at = Some(scheduled_at.into());
        self
    }

    /// Set a field this struct does not name.
    pub fn extra(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }
}

/// A reference to a published server template, plus the variables to render it
/// with.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TemplateRef {
    pub id: String,
    #[serde(skip_serializing_if = "Map::is_empty", default)]
    pub variables: Map<String, Value>,
}

impl TemplateRef {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            variables: Map::new(),
        }
    }

    pub fn variable(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }
}

/// An arbitrary label carried with the send and echoed back on its events.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub value: String,
}

impl Tag {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// A file to attach. `content` is base64 — [`Attachment::from_bytes`] does the
/// encoding for you.
///
/// Set `content_type` and a `content_id` to embed an inline image referenced by
/// `cid:` in the HTML; omit `content_id` for a regular file attachment.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Attachment {
    pub filename: String,
    /// Base64-encoded bytes.
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub content_id: Option<String>,
}

impl Attachment {
    /// From bytes you already hold — a PDF you rendered, a file you read.
    pub fn from_bytes(filename: impl Into<String>, content: impl AsRef<[u8]>) -> Self {
        use base64::Engine as _;
        Self {
            filename: filename.into(),
            content: base64::engine::general_purpose::STANDARD.encode(content),
            content_type: None,
            content_id: None,
        }
    }

    /// From content you have already base64-encoded.
    pub fn from_base64(filename: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            filename: filename.into(),
            content: content.into(),
            content_type: None,
            content_id: None,
        }
    }

    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    /// Make it an inline image, referenced as `cid:<content_id>` in the HTML.
    pub fn content_id(mut self, content_id: impl Into<String>) -> Self {
        self.content_id = Some(content_id.into());
        self
    }
}

/// What a send returns: the id to look the email up by.
#[derive(Clone, Debug, Deserialize)]
pub struct SentEmail {
    pub id: String,
    /// Everything else the API sent — a replayed idempotent send answers with
    /// the original body, which carries more than the id.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// What [`Emails::batch`] returns: one id per message, in the order sent.
#[derive(Clone, Debug, Deserialize)]
pub struct BatchSent {
    pub data: Vec<SentEmail>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A stored email. Unknown fields land in `extra` rather than failing the
/// deserialize, so a field added server-side will not break this.
#[derive(Clone, Debug, Deserialize)]
pub struct Email {
    pub id: String,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    /// The delivered envelope. A string for a single recipient, an array for
    /// several, so it is kept as JSON.
    #[serde(default)]
    pub to: Option<Value>,
    #[serde(default)]
    pub subject: Option<String>,
    /// The latest thing that happened: `queued`, `scheduled`, `sent`,
    /// `delivered`, `delivery_delayed`, `bounced`, `complained`, `suppressed`,
    /// `canceled`, `failed`. A `String` rather than an enum on purpose — a
    /// status added server-side should not stop this from deserializing.
    #[serde(default)]
    pub last_event: Option<String>,
    /// The friendly alias of `last_event`. [`Emails::get`] fills it in when the
    /// API does not send one.
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub scheduled_at: Option<String>,
    /// Why the send failed, when it did. The API redacts this before it leaves
    /// the building, so it is a reason to show an operator, not a provider
    /// diagnostic to parse.
    #[serde(default)]
    pub error: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
