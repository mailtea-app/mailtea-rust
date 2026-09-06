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
///
/// [`create`](Domains::create) takes `region` (fixed at creation), `tls` and
/// `tracking_subdomain`; [`list`](Domains::list) filters on `region` and
/// `status`.
#[derive(Clone, Debug)]
pub struct Domains {
    inner: Arc<Inner>,
    /// Tracking sub-domains (CNAME) under a domain.
    pub tracking: TrackingDomains,
    /// Domain claims — take a domain back from another publication.
    pub claims: DomainClaims,
}

impl Domains {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self {
            tracking: TrackingDomains::new(Arc::clone(&inner)),
            claims: DomainClaims::new(Arc::clone(&inner)),
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
    ///
    /// `"tracking_subdomain": null` removes a tracking subdomain: the domain's
    /// links go back to being served from the Mailtea host, and links in mail
    /// already sent point at the old hostname and stop resolving. The payload
    /// is serialized as given, so the null reaches the wire — omitting the key
    /// (leave the subdomain alone) and passing null (remove it) are different
    /// requests, and a params type that skips its `None`s would send neither.
    /// An empty string is not a third spelling; it is refused with
    /// `tracking_subdomain_invalid`. null is an update-only value: a create has
    /// nothing to clear.
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

/// Domain claims — take a domain back from whichever publication currently
/// holds it. Reached through [`Mailtea::domains`](crate::Mailtea::domains)`.claims`.
///
/// Use this when adding a domain is refused because the host is connected to
/// another publication: open a claim, publish the TXT record the response lists
/// to prove you control the DNS, then [`verify`](DomainClaims::verify). On
/// success the other team's domain is released and a fresh one is created for
/// you.
#[derive(Clone, Debug)]
pub struct DomainClaims {
    inner: Arc<Inner>,
}

impl DomainClaims {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// `POST /v1/domains/claim` — open a claim. Takes `publication_id`, `name`
    /// and an optional `region`; the response `records` lists the TXT record to
    /// publish.
    pub async fn create(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call("POST", "/v1/domains/claim", crate::params::to_body(params)?)
            .await
    }

    /// `GET /v1/domains/claims/:id` — poll a claim. Requires `publication_id`.
    pub async fn get(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!(
                    "/v1/domains/claims/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `POST /v1/domains/claims/:id/verify` — check the TXT record and complete
    /// the claim if it is there.
    ///
    /// Safe to call repeatedly: a record that has not propagated yet leaves the
    /// claim pending with the same record, so nothing has to be republished. A
    /// completed claim answers with the fresh `domain` beside the claim.
    pub async fn verify(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "POST",
                &format!(
                    "/v1/domains/claims/{}/verify{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `DELETE /v1/domains/claims/:id` — withdraw a pending claim. Requires
    /// `publication_id`.
    pub async fn cancel(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "DELETE",
                &format!(
                    "/v1/domains/claims/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }
}
