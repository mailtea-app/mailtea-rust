use std::fmt;

use serde_json::Value;

/// The result of every SDK call.
pub type Result<T> = std::result::Result<T, Error>;

/// Something went wrong: the API refused the request, or the client could not
/// make it in the first place.
///
/// One type rather than an enum, because every caller needs the same five
/// things and branching happens on [`status`](Error::status) or
/// [`code`](Error::code), not on a variant.
///
/// ```no_run
/// # async fn run(mailtea: mailtea::Mailtea, email: mailtea::SendEmail) {
/// match mailtea.emails.send(&email).await {
///     Ok(sent) => println!("Sent {}", sent.id),
///     Err(error) if error.is_client_error() => {
///         // The request never landed — DNS, TLS, connection, timeout, or a
///         // missing key. Nothing was sent.
///         eprintln!("{error}");
///     }
///     Err(error) => {
///         // Mailtea answered and refused. `details` names the fields a 400
///         // objected to; "Validation failed" alone does not say what to change.
///         eprintln!("HTTP {}: {}", error.status(), error.message());
///         if let Some(details) = error.details() {
///             eprintln!("{details}");
///         }
///     }
/// }
/// # }
/// ```
///
/// The struct is one pointer wide, so `Result<T, Error>` stays cheap to return.
pub struct Error {
    inner: Box<Inner>,
}

struct Inner {
    status: u16,
    message: String,
    code: Option<String>,
    details: Option<Value>,
    request_id: Option<String>,
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

impl Error {
    /// A fault on this side of the wire: nothing was sent.
    pub(crate) fn client(message: impl Into<String>, code: &str) -> Self {
        Self {
            inner: Box::new(Inner {
                status: 0,
                message: message.into(),
                code: Some(code.to_string()),
                details: None,
                request_id: None,
                source: None,
            }),
        }
    }

    pub(crate) fn with_source(
        mut self,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        self.inner.source = Some(Box::new(source));
        self
    }

    /// The API answered and refused.
    pub(crate) fn api(
        status: u16,
        message: impl Into<String>,
        code: Option<String>,
        details: Option<Value>,
        request_id: Option<String>,
    ) -> Self {
        Self {
            inner: Box::new(Inner {
                status,
                message: message.into(),
                code,
                details,
                request_id,
                source: None,
            }),
        }
    }

    /// HTTP status. **`0` means the request never reached the API** — a missing
    /// key, a DNS or TLS failure, a timeout, or a response body that did not
    /// parse. Nothing was sent, so a retry is safe.
    pub fn status(&self) -> u16 {
        self.inner.status
    }

    /// The API's own `error` string, or a description of the client-side fault.
    pub fn message(&self) -> &str {
        &self.inner.message
    }

    /// Machine-readable code, when there is one (`marketing_plan_required` on a
    /// 402, `missing_api_key` client-side). Branching on this survives a copy
    /// change to [`message`](Error::message).
    pub fn code(&self) -> Option<&str> {
        self.inner.code.as_deref()
    }

    /// The `details` array a 400 carries — one entry per field the API refused.
    pub fn details(&self) -> Option<&Value> {
        self.inner.details.as_ref()
    }

    /// The `x-request-id` header. Quote it in a support request.
    pub fn request_id(&self) -> Option<&str> {
        self.inner.request_id.as_deref()
    }

    /// True when the request never reached the API ([`status`](Error::status)
    /// is `0`), so nothing was sent and repeating it cannot deliver twice.
    pub fn is_client_error(&self) -> bool {
        self.inner.status == 0
    }

    /// True for the statuses worth backing off and retrying: 429 and 5xx. A
    /// retried send should carry the same `Idempotency-Key` as the first
    /// attempt — without one, a retry after a lost answer sends twice.
    pub fn is_retryable(&self) -> bool {
        self.inner.status == 429 || (500..600).contains(&self.inner.status)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error")
            .field("status", &self.inner.status)
            .field("message", &self.inner.message)
            .field("code", &self.inner.code)
            .field("details", &self.inner.details)
            .field("request_id", &self.inner.request_id)
            .finish()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.inner.status == 0 {
            write!(f, "Mailtea client error: {}", self.inner.message)?;
        } else {
            write!(
                f,
                "Mailtea API error (HTTP {}): {}",
                self.inner.status, self.inner.message
            )?;
            if let Some(code) = &self.inner.code {
                write!(f, " [{code}]")?;
            }
        }
        if let Some(request_id) = &self.inner.request_id {
            write!(f, " (request id {request_id})")?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner
            .source
            .as_ref()
            .map(|boxed| boxed.as_ref() as &(dyn std::error::Error + 'static))
    }
}
