use std::env;
use std::fmt;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::http::{HttpRequest, HttpResponse, ReqwestTransport, Transport};
use crate::params;
use crate::resources::{
    ApiKeys, Assets, AutomationRuns, Automations, ContactProperties, Contacts, Domains, Emails,
    EventDefinitions, Events, Posts, Segments, Senders, Suppressions, Templates, Topics, Webhooks,
};

/// Production API. Override it for local dev or a self-hosted Mailtea.
pub const DEFAULT_BASE_URL: &str = "https://api.mailtea.app";

/// The crate version, sent as the `User-Agent` on every request.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The Mailtea client.
///
/// ```no_run
/// # async fn run() -> mailtea::Result<()> {
/// use mailtea::{Mailtea, SendEmail};
///
/// let mailtea = Mailtea::from_env()?; // reads MAILTEA_API_KEY
///
/// let sent = mailtea
///     .emails
///     .send(&SendEmail::new(
///         "you@yourdomain.com",
///         ["recipient@example.com"],
///         "Hello from Mailtea",
///     ).html("<p>Your first email, sent with Mailtea.</p>"))
///     .await?;
///
/// println!("{}", sent.id);
/// # Ok(())
/// # }
/// ```
///
/// Cloning is cheap — every resource shares one `Arc` of connection pool and
/// configuration — so a `Mailtea` can be stored in application state and cloned
/// per request.
#[derive(Clone)]
pub struct Mailtea {
    /// Transactional email: send, schedule, cancel, and inbound.
    pub emails: Emails,
    /// Audience contacts.
    pub contacts: Contacts,
    /// Newsletter posts (issues and broadcasts).
    pub posts: Posts,
    /// Audience segments.
    pub segments: Segments,
    /// Named From identities.
    pub senders: Senders,
    /// A publication's image library.
    pub assets: Assets,
    /// The team-wide do-not-send list.
    pub suppressions: Suppressions,
    /// Topic definitions.
    pub topics: Topics,
    /// Reusable server-side email templates.
    pub templates: Templates,
    /// Sending domains and their tracking sub-domains.
    pub domains: Domains,
    /// Outbound webhook subscriptions.
    pub webhooks: Webhooks,
    /// Custom contact fields (team-scoped).
    pub contact_properties: ContactProperties,
    /// API keys.
    pub api_keys: ApiKeys,
    /// Multi-step contact journeys.
    pub automations: Automations,
    /// One contact's journey through one automation.
    pub automation_runs: AutomationRuns,
    /// Custom product events.
    pub events: Events,
    /// The catalog of event names a publication expects.
    pub event_definitions: EventDefinitions,
    inner: Arc<Inner>,
}

impl Mailtea {
    /// A client for the given key, against `https://api.mailtea.app` — or
    /// whatever `MAILTEA_API_BASE_URL` names, for local dev and self-hosting.
    ///
    /// # Panics
    /// If the TLS backend cannot start, which is what `reqwest::Client::new()`
    /// panics on too. Use [`Mailtea::builder`] with your own
    /// [`Transport`](crate::Transport) to avoid it.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::builder()
            .api_key(api_key)
            .build()
            .expect("an explicit API key is all `build` requires")
    }

    /// A client reading `MAILTEA_API_KEY` (and optionally
    /// `MAILTEA_API_BASE_URL`) from the environment.
    ///
    /// Returns a `status: 0` error with code `missing_api_key` when the
    /// variable is unset or empty — a misconfiguration, not a failed request.
    pub fn from_env() -> Result<Self> {
        Self::builder().build()
    }

    /// Set the base URL, or supply your own [`Transport`](crate::Transport).
    pub fn builder() -> MailteaBuilder {
        MailteaBuilder::default()
    }

    /// The base URL this client talks to, trailing slash already stripped.
    pub fn base_url(&self) -> &str {
        &self.inner.base_url
    }

    /// Call an endpoint this SDK does not wrap yet, or deserialize a wrapped
    /// one into your own type.
    ///
    /// ```no_run
    /// # async fn run(mailtea: mailtea::Mailtea) -> mailtea::Result<()> {
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct Sender { id: String, email: String }
    /// #[derive(Deserialize)]
    /// struct SenderList { data: Vec<Sender> }
    ///
    /// let list: SenderList = mailtea
    ///     .request("GET", "/v1/senders?publication_id=pub_123", ())
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// `path` must already carry its query string; pass `()` for no body.
    pub async fn request<T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: impl Serialize,
    ) -> Result<T> {
        self.inner.call(method, path, params::to_body(body)?).await
    }
}

// Hand-written so a stray `dbg!(&client)` cannot print the API key.
impl fmt::Debug for Mailtea {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mailtea")
            .field("base_url", &self.inner.base_url)
            .field("api_key", &"<redacted>")
            .finish()
    }
}

/// Builds a [`Mailtea`] with a base URL or a transport of your own.
#[derive(Default)]
pub struct MailteaBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
    transport: Option<Arc<dyn Transport>>,
}

// Hand-written for the same reason `Mailtea`'s is: a builder holding a key
// should not print it.
impl fmt::Debug for MailteaBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MailteaBuilder")
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("base_url", &self.base_url)
            .field("transport", &self.transport.as_ref().map(|_| "<custom>"))
            .finish()
    }
}

impl MailteaBuilder {
    /// The API key. Omit it and `MAILTEA_API_KEY` is read instead.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Point at a self-hosted or local Mailtea. Omit it and
    /// `MAILTEA_API_BASE_URL` is read, falling back to [`DEFAULT_BASE_URL`].
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Send requests through something other than the bundled `reqwest`
    /// client — a mock in tests, or your own instrumented HTTP stack.
    pub fn transport(mut self, transport: impl Transport + 'static) -> Self {
        self.transport = Some(Arc::new(transport));
        self
    }

    /// Build the client, or fail with `missing_api_key`.
    pub fn build(self) -> Result<Mailtea> {
        let api_key = self
            .api_key
            .or_else(|| env::var("MAILTEA_API_KEY").ok())
            .filter(|key| !key.is_empty())
            .ok_or_else(|| {
                Error::client(
                    "Missing Mailtea API key. Pass it to Mailtea::new(api_key) or set the \
                     MAILTEA_API_KEY environment variable.",
                    "missing_api_key",
                )
            })?;

        let base_url = self
            .base_url
            .or_else(|| env::var("MAILTEA_API_BASE_URL").ok())
            .filter(|url| !url.is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        let inner = Arc::new(Inner {
            api_key,
            // A trailing slash in the env var would otherwise produce `//v1/emails`.
            base_url: base_url.trim_end_matches('/').to_string(),
            transport: self
                .transport
                .unwrap_or_else(|| Arc::new(ReqwestTransport::new())),
        });

        Ok(Mailtea {
            emails: Emails::new(Arc::clone(&inner)),
            contacts: Contacts::new(Arc::clone(&inner)),
            posts: Posts::new(Arc::clone(&inner)),
            segments: Segments::new(Arc::clone(&inner)),
            senders: Senders::new(Arc::clone(&inner)),
            assets: Assets::new(Arc::clone(&inner)),
            suppressions: Suppressions::new(Arc::clone(&inner)),
            topics: Topics::new(Arc::clone(&inner)),
            templates: Templates::new(Arc::clone(&inner)),
            domains: Domains::new(Arc::clone(&inner)),
            webhooks: Webhooks::new(Arc::clone(&inner)),
            contact_properties: ContactProperties::new(Arc::clone(&inner)),
            api_keys: ApiKeys::new(Arc::clone(&inner)),
            automations: Automations::new(Arc::clone(&inner)),
            automation_runs: AutomationRuns::new(Arc::clone(&inner)),
            events: Events::new(Arc::clone(&inner)),
            event_definitions: EventDefinitions::new(Arc::clone(&inner)),
            inner,
        })
    }
}

/// Everything the resources share. One per client, behind an `Arc`.
pub(crate) struct Inner {
    api_key: String,
    base_url: String,
    transport: Arc<dyn Transport>,
}

impl fmt::Debug for Inner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mailtea")
            .field("base_url", &self.base_url)
            .field("api_key", &"<redacted>")
            .finish()
    }
}

impl Inner {
    /// The one place a request is built, sent, and turned into a `Result`.
    async fn execute(
        &self,
        method: &str,
        path: &str,
        body: Option<Vec<u8>>,
        extra_headers: &[(String, String)],
    ) -> Result<HttpResponse> {
        let mut headers = vec![
            (
                "Authorization".to_string(),
                format!("Bearer {}", self.api_key),
            ),
            ("User-Agent".to_string(), format!("mailtea-rust/{VERSION}")),
            ("Accept".to_string(), "application/json".to_string()),
        ];
        if body.is_some() {
            headers.push(("Content-Type".to_string(), "application/json".to_string()));
        }
        headers.extend_from_slice(extra_headers);

        let response = self
            .transport
            .execute(HttpRequest {
                method: method.to_string(),
                url: format!("{}{path}", self.base_url),
                headers,
                body,
            })
            .await?;

        if response.status >= 400 {
            return Err(api_error(&response));
        }
        Ok(response)
    }

    pub(crate) async fn call<T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<Vec<u8>>,
    ) -> Result<T> {
        self.call_with_headers(method, path, body, &[]).await
    }

    pub(crate) async fn call_with_headers<T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<Vec<u8>>,
        extra_headers: &[(String, String)],
    ) -> Result<T> {
        let response = self.execute(method, path, body, extra_headers).await?;
        // A 204 (or any empty 2xx) is `null`, which deserializes into
        // `serde_json::Value`, `Option<T>` and `()` alike.
        let text = if response.body.trim().is_empty() {
            "null"
        } else {
            &response.body
        };
        serde_json::from_str(text).map_err(|error| {
            Error::client(
                format!("unexpected response from Mailtea: {error}"),
                "invalid_response",
            )
            .with_source(error)
        })
    }

    /// For the endpoints that answer with something other than JSON —
    /// `GET /v1/suppressions/export` returns CSV.
    pub(crate) async fn call_text(
        &self,
        method: &str,
        path: &str,
        body: Option<Vec<u8>>,
    ) -> Result<String> {
        Ok(self.execute(method, path, body, &[]).await?.body)
    }
}

/// Turn a non-2xx answer into an [`Error`].
///
/// Errors come back as `{"error": ..., "details"?: [...], "code"?: ...}` with an
/// `x-request-id` header. A body that is not JSON at all — a proxy's HTML 502 —
/// keeps the status line as the message rather than being dropped.
fn api_error(response: &HttpResponse) -> Error {
    let request_id = response.header("x-request-id").map(str::to_string);
    let mut message = format!("HTTP {}", response.status);
    let mut code = None;
    let mut details = None;

    if let Ok(Value::Object(parsed)) = serde_json::from_str::<Value>(&response.body) {
        if let Some(error) = parsed.get("error").and_then(Value::as_str) {
            if !error.is_empty() {
                message = error.to_string();
            }
        }
        code = parsed
            .get("code")
            .and_then(Value::as_str)
            .map(str::to_string);
        details = parsed.get("details").cloned().filter(|d| !d.is_null());
    }

    Error::api(response.status, message, code, details, request_id)
}
