use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use crate::client::Inner;
use crate::error::Result;
use crate::params;

const BASE: &str = "/v1/emails/inbound";

/// Inbound (received) emails. Reached through
/// [`Mailtea::emails`](crate::Mailtea::emails)`.inbound`.
///
/// List and retrieve mail delivered to your receiving domains, download
/// attachments, and [`reply`](InboundEmails::reply) — which threads correctly by
/// construction and reuses the transactional send pipeline. Scoped to a
/// publication: pass `publication_id` to [`list`](InboundEmails::list).
#[derive(Clone, Debug)]
pub struct InboundEmails {
    inner: Arc<Inner>,
    /// Attachments on a received email.
    pub attachments: InboundAttachments,
}

impl InboundEmails {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self {
            attachments: InboundAttachments::new(Arc::clone(&inner)),
            inner,
        }
    }

    /// `GET /v1/emails/inbound` — received emails in a publication, most recent
    /// first, cursor-paginated. Takes `publication_id`, optional `limit` (1-100,
    /// default 20) and `cursor`.
    pub async fn list(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call("GET", &format!("{BASE}{}", params::query(params)?), None)
            .await
    }

    /// `GET /v1/emails/inbound/:id` — one received email, including its body,
    /// headers, and attachments.
    pub async fn get(&self, id: &str) -> Result<Value> {
        self.inner
            .call("GET", &format!("{BASE}/{}", params::encode(id)), None)
            .await
    }

    /// `POST /v1/emails/inbound/:id/reply` — reply to a received email.
    ///
    /// The reply target (`to`), threading headers, and the `Re: ` subject
    /// default are all server-derived — pass only the content (`html`/`text`,
    /// and optionally `from`, `subject`, `cc`, `bcc`, `idempotency_key`).
    /// Returns the resulting transactional email's `id` and `status`.
    pub async fn reply(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "POST",
                &format!("{BASE}/{}/reply", crate::params::encode(id)),
                crate::params::to_body(params)?,
            )
            .await
    }
}

/// Attachments on a received email. Each carries a short-lived signed
/// `download_url`.
#[derive(Clone, Debug)]
pub struct InboundAttachments {
    inner: Arc<Inner>,
}

impl InboundAttachments {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// `GET /v1/emails/inbound/:id/attachments` — every attachment, each with a
    /// signed download URL.
    pub async fn list(&self, id: &str) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!("{BASE}/{}/attachments", params::encode(id)),
                None,
            )
            .await
    }

    /// `GET /v1/emails/inbound/:id/attachments/:attachment_id` — one
    /// attachment with a signed download URL.
    pub async fn get(&self, id: &str, attachment_id: &str) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!(
                    "{BASE}/{}/attachments/{}",
                    params::encode(id),
                    params::encode(attachment_id)
                ),
                None,
            )
            .await
    }
}
