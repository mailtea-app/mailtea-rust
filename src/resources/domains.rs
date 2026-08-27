use std::sync::Arc;

use serde::Serialize;
use serde_json::{json, Value};

use crate::client::Inner;
use crate::error::Result;
use crate::params;

/// The `domains` resource (email and site sending domains). Reached through
/// [`Mailtea::domains`](crate::Mailtea::domains).
///
/// Scoped to a publication — pass `publication_id`. Register a domain, add the
/// returned DNS `records`, then [`verify`](Domains::verify) it before sending
/// from it.
#[derive(Clone, Debug)]
pub struct Domains {
    inner: Arc<Inner>,
    /// Tracking sub-domains (CNAME) under a domain.
    pub tracking: TrackingDomains,
}

impl Domains {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self {
            tracking: TrackingDomains::new(Arc::clone(&inner)),
            inner,
        }
    }

    /// `POST /v1/domains` — register a domain. The response `records` lists the
    /// DNS records to add.
    pub async fn create(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call("POST", "/v1/domains", crate::params::to_body(params)?)
            .await
    }

    /// `GET /v1/domains` — list domains. Requires `publication_id`.
    pub async fn list(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!("/v1/domains{}", params::query(params)?),
                None,
            )
            .await
    }

    /// `GET /v1/domains/:id` — one domain. Requires `publication_id`.
    pub async fn get(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!(
                    "/v1/domains/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `POST /v1/domains/:id/verify` — verify a domain via its DNS records;
    /// `status` becomes `"verified"`.
    pub async fn verify(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "POST",
                &format!(
                    "/v1/domains/{}/verify{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `PATCH /v1/domains/:id` — update a domain, including
    /// `custom_return_path` (delegate a subdomain as the envelope sender so SPF
    /// aligns with your own domain). `publication_id` rides in both the query
    /// and the body.
    pub async fn update(&self, id: &str, params: impl Serialize) -> Result<Value> {
        let payload = crate::params::to_value(params)?;
        self.inner
            .call(
                "PATCH",
                &format!(
                    "/v1/domains/{}{}",
                    crate::params::encode(id),
                    crate::params::query_pick(&payload, &["publication_id"])
                ),
                crate::params::body_from_value(&payload)?,
            )
            .await
    }

    /// `DELETE /v1/domains/:id` — remove a domain. Requires `publication_id`.
    pub async fn delete(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "DELETE",
                &format!(
                    "/v1/domains/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }
}

/// Tracking sub-domains (CNAME) under a domain — used to serve the open pixel
/// and click-tracking links from your own domain. Reached through
/// [`Mailtea::domains`](crate::Mailtea::domains)`.tracking`.
#[derive(Clone, Debug)]
pub struct TrackingDomains {
    inner: Arc<Inner>,
}

impl TrackingDomains {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// `POST /v1/domains/:domain_id/tracking-domains` — add a tracking
    /// sub-domain. Takes `publication_id` and `subdomain`; the response
    /// `records` lists the CNAME to add.
    ///
    /// `publication_id` goes in the query and only `subdomain` in the body,
    /// which is what the endpoint's schema accepts.
    pub async fn create(&self, domain_id: &str, params: impl Serialize) -> Result<Value> {
        let payload = crate::params::to_value(params)?;
        let subdomain = payload.get("subdomain").cloned().unwrap_or(Value::Null);
        self.inner
            .call(
                "POST",
                &format!(
                    "/v1/domains/{}/tracking-domains{}",
                    crate::params::encode(domain_id),
                    crate::params::query_pick(&payload, &["publication_id"])
                ),
                crate::params::body_from_value(&json!({ "subdomain": subdomain }))?,
            )
            .await
    }

    /// `GET /v1/domains/:domain_id/tracking-domains` — list them. Requires
    /// `publication_id`.
    pub async fn list(&self, domain_id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!(
                    "/v1/domains/{}/tracking-domains{}",
                    crate::params::encode(domain_id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `POST /v1/domains/:domain_id/tracking-domains/:id/verify` — verify the
    /// CNAME. Requires `publication_id`.
    pub async fn verify(
        &self,
        domain_id: &str,
        tracking_domain_id: &str,
        params: impl Serialize,
    ) -> Result<Value> {
        self.inner
            .call(
                "POST",
                &format!(
                    "/v1/domains/{}/tracking-domains/{}/verify{}",
                    crate::params::encode(domain_id),
                    crate::params::encode(tracking_domain_id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `DELETE /v1/domains/:domain_id/tracking-domains/:id` — remove it.
    /// Requires `publication_id`.
    pub async fn delete(
        &self,
        domain_id: &str,
        tracking_domain_id: &str,
        params: impl Serialize,
    ) -> Result<Value> {
        self.inner
            .call(
                "DELETE",
                &format!(
                    "/v1/domains/{}/tracking-domains/{}{}",
                    crate::params::encode(domain_id),
                    crate::params::encode(tracking_domain_id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }
}
