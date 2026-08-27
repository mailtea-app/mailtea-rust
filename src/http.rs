//! The transport seam.
//!
//! Everything above this file builds a [`HttpRequest`] and reads a
//! [`HttpResponse`]; nothing above it knows what `reqwest` is. That is what
//! makes the tests in `tests/` run with no credentials and no network — they
//! hand the client a [`Transport`] of their own.

use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use crate::error::{Error, Result};

/// A boxed future, because a trait with `async fn` cannot be used as
/// `dyn Transport` and an injectable transport has to be a trait object.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// One outgoing request, already fully addressed and authorized.
#[derive(Clone, Debug)]
pub struct HttpRequest {
    /// Uppercase HTTP method: `GET`, `POST`, `PATCH`, `DELETE`.
    pub method: String,
    /// The absolute URL, base included.
    pub url: String,
    /// Header name/value pairs, `Authorization` and `User-Agent` among them.
    pub headers: Vec<(String, String)>,
    /// The JSON body, or `None` for a request that carries none.
    pub body: Option<Vec<u8>>,
}

/// One answer. The body is kept as text: an error response carries JSON the
/// caller needs to see, and deserializing straight into the success type would
/// throw it away.
#[derive(Clone, Debug)]
pub struct HttpResponse {
    pub status: u16,
    /// Header names lowercased, which is how [`HttpResponse::header`] looks
    /// them up.
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpResponse {
    /// Look up a header by its (case-insensitive) name.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }
}

/// How a request actually goes out. Implement it to test without a network, to
/// add retries or tracing, or to route through your own HTTP stack.
///
/// ```no_run
/// use mailtea::{BoxFuture, HttpRequest, HttpResponse, Mailtea, Result, Transport};
/// use std::collections::HashMap;
///
/// #[derive(Debug)]
/// struct AlwaysAccepts;
///
/// impl Transport for AlwaysAccepts {
///     fn execute<'a>(&'a self, _request: HttpRequest) -> BoxFuture<'a, Result<HttpResponse>> {
///         Box::pin(async {
///             Ok(HttpResponse {
///                 status: 200,
///                 headers: HashMap::new(),
///                 body: r#"{"id":"txemail_1"}"#.to_string(),
///             })
///         })
///     }
/// }
///
/// let mailtea = Mailtea::builder()
///     .api_key("mt_pat_test")
///     .transport(AlwaysAccepts)
///     .build()
///     .unwrap();
/// ```
pub trait Transport: Send + Sync + fmt::Debug {
    fn execute<'a>(&'a self, request: HttpRequest) -> BoxFuture<'a, Result<HttpResponse>>;
}

impl<T: Transport + ?Sized> Transport for Arc<T> {
    fn execute<'a>(&'a self, request: HttpRequest) -> BoxFuture<'a, Result<HttpResponse>> {
        (**self).execute(request)
    }
}

/// How long a single call may take before it is given up on.
///
/// `reqwest::Client::new()` has no timeout at all, and a send that hangs
/// forever is worse than one that fails: nothing retries it and nothing logs
/// it. Thirty seconds is generous for this API and still bounded.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// The default transport: `reqwest` over rustls.
#[derive(Clone, Debug)]
pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    /// A client with the SDK's 30-second timeout.
    ///
    /// # Panics
    /// If the TLS backend cannot start — the same condition `reqwest::Client::new()`
    /// panics on.
    pub fn new() -> Self {
        Self::with_client(
            reqwest::Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()
                .expect("could not build the HTTP client"),
        )
    }

    /// Bring your own `reqwest::Client` — connection pool, proxy, timeouts and
    /// all. Nothing here is overridden, so set a timeout on it.
    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }
}

impl Default for ReqwestTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for ReqwestTransport {
    fn execute<'a>(&'a self, request: HttpRequest) -> BoxFuture<'a, Result<HttpResponse>> {
        Box::pin(async move {
            let method =
                reqwest::Method::from_bytes(request.method.as_bytes()).map_err(|error| {
                    Error::client(
                        format!("invalid HTTP method {:?}", request.method),
                        "invalid_request",
                    )
                    .with_source(error)
                })?;

            let mut builder = self.client.request(method, &request.url);
            for (name, value) in &request.headers {
                builder = builder.header(name, value);
            }
            if let Some(body) = request.body {
                builder = builder.body(body);
            }

            let response = builder.send().await.map_err(|error| {
                Error::client(
                    format!("could not reach the Mailtea API: {error}"),
                    "transport_error",
                )
                .with_source(error)
            })?;

            let status = response.status().as_u16();
            let headers = response
                .headers()
                .iter()
                .map(|(name, value)| {
                    (
                        name.as_str().to_ascii_lowercase(),
                        value.to_str().unwrap_or_default().to_string(),
                    )
                })
                .collect();
            let body = response.text().await.map_err(|error| {
                Error::client(
                    format!("could not read the Mailtea API response: {error}"),
                    "transport_error",
                )
                .with_source(error)
            })?;

            Ok(HttpResponse {
                status,
                headers,
                body,
            })
        })
    }
}
