//! The official Rust SDK for [Mailtea](https://mailtea.app) — a thin, typed
//! wrapper over the [REST API](https://docs.mailtea.app/docs/api-reference).
//!
//! ```no_run
//! use mailtea::{Mailtea, SendEmail};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Reads MAILTEA_API_KEY, and MAILTEA_API_BASE_URL when it is set.
//!     let mailtea = Mailtea::from_env()?;
//!
//!     let sent = mailtea
//!         .emails
//!         .send(
//!             &SendEmail::new(
//!                 "you@yourdomain.com",
//!                 ["recipient@example.com"],
//!                 "Hello from Mailtea",
//!             )
//!             .html("<p>Your first email, sent with <strong>Mailtea</strong>.</p>"),
//!         )
//!         .await?;
//!
//!     let email = mailtea.emails.get(&sent.id).await?;
//!     println!("{} {:?}", email.id, email.status);
//!     Ok(())
//! }
//! ```
//!
//! # Payloads
//!
//! Every method takes `impl Serialize`, so a payload can be a typed request
//! struct ([`SendEmail`], [`CreateContact`], [`CreatePost`], [`CreateTopic`],
//! [`SendTestPost`], [`UploadAsset`], …), a `serde_json::json!({...})` literal,
//! or any struct of your own. Pass `()` where a method takes parameters you do
//! not need.
//!
//! Responses are typed on the transactional email path ([`SentEmail`],
//! [`Email`], [`BatchSent`]) and `serde_json::Value` elsewhere. To deserialize
//! any endpoint into your own type, call [`Mailtea::request`].
//!
//! # Errors
//!
//! Every call returns [`Result<T>`], whose error is [`Error`] — one struct
//! carrying the HTTP `status`, the API's `message`, its `code` and `details`
//! when present, and the `x-request-id`. A `status` of `0` means the request
//! never reached the API, so nothing was sent.
//!
//! # Webhooks
//!
//! [`verify_webhook_signature`] checks a Standard Webhooks delivery and rejects
//! replays; [`sign_webhook`] produces the same header for tests.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

mod client;
mod error;
mod http;
mod params;
mod resources;
mod webhook;

pub use client::{Mailtea, MailteaBuilder, DEFAULT_BASE_URL, VERSION};
pub use error::{Error, Result};
pub use http::{BoxFuture, HttpRequest, HttpResponse, ReqwestTransport, Transport};
pub use resources::*;
pub use webhook::{
    sign_webhook, verify_webhook_signature, verify_webhook_signature_with, IntoUnixSeconds,
    VerifyOptions, DEFAULT_TOLERANCE_SECONDS,
};
