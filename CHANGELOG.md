# Changelog

All notable changes to the `mailtea` Rust crate are documented here.

## Unreleased

- Added: `domains.update` with `"tracking_subdomain": null` removes a tracking
  subdomain. The domain's links go back to being served from the Mailtea host.
  Links in mail you have already sent point at the old hostname and stop
  resolving — there is no way to reinstate them. The payload is serialized as
  given, so the null reaches the wire; omitting the key and passing null are
  different requests, and a params type that skips its `None`s sends neither.
  An empty string is neither: it is refused with `tracking_subdomain_invalid`.
- Changed: the `MX` row in `records` now reports what the last verify found,
  instead of reading `pending` on every request but the verify itself. A domain
  nobody has verified reads `not_started`.

## 0.2.0 (2026-09-03)

- Added: the domain claims resource — `mailtea.domains.claims.create`, `.get`,
  `.verify` and `.cancel`. When adding a domain is refused because the host is
  connected to another publication, publish one TXT record to prove you control
  its DNS and the domain moves to you.
- Documented: domains take `region` (fixed at creation), `tls` and
  `tracking_subdomain` on create, and the list filters on `region` and `status`.
  This SDK forwards whatever parameters you pass, so these worked already — this
  release is where they are stated and covered by tests.

## 0.1.0 (2026-08-27)

First release. A thin, typed, async wrapper over the Mailtea REST API, at
endpoint parity with the Node.js and Python SDKs.

### Added

- **`Mailtea`** — the client. `Mailtea::new(key)`, `Mailtea::from_env()` (reads
  `MAILTEA_API_KEY`), or `Mailtea::builder()` for a base URL
  (`MAILTEA_API_BASE_URL`, or explicit; a trailing slash is stripped) and a
  transport of your own. Cloning is cheap — every resource shares one
  connection pool — so it belongs in application state. `Debug` prints the key
  redacted.

- **Every resource the other SDKs reach**, method for method and path for path:
  `emails` (`send`, `send_idempotent`, `batch`, `get`, `list`, `analytics`,
  `update`, `reschedule`, `cancel`, and the `inbound` sub-resource with its
  `attachments`), `contacts`, `segments`, `topics`, `posts`, `senders`,
  `assets`, `suppressions` (including `export`, which returns CSV, not JSON),
  `templates`, `domains` (plus `domains.tracking`), `webhooks`,
  `contact_properties`, `api_keys`, `automations`, `automation_runs`, `events`
  and `event_definitions`. `tests/endpoint_parity.rs` pins the list and fails if
  one goes missing — or if this crate invents one.

- **Typed request structs** where a compile-time check earns its keep:
  `SendEmail` (with `Tag`, `Attachment`, `TemplateRef`), `CreateContact`,
  `UpdateContact`, `CreatePost`, `SendPost`, `SendTestPost`, `CreateTopic` and
  `UploadAsset`. Each carries an `extra` map flattened into the same JSON
  object, so a field the API grows before this crate does still reaches the
  wire. Every method takes `impl Serialize`, so a `serde_json::json!` payload
  works everywhere a struct does — which is how the long tail of filters and
  automation graphs is passed.

- **`Error`** — one struct carrying `status()`, `message()`, `code()`,
  `details()` and `request_id()`. A `status()` of `0` means the request never
  reached the API, so nothing was sent; `is_retryable()` covers 429 and 5xx.

- **`Transport`** — the seam every request goes through. `ReqwestTransport` is
  the default (rustls, 30-second per-request timeout); implement the trait to
  test without a network, add retries or tracing, or route through your own
  stack.

- **Webhook signature verification** — `verify_webhook_signature`,
  `verify_webhook_signature_with` (tolerance and clock injectable) and
  `sign_webhook`, a port of the platform's Standard Webhooks signer. Constant-
  time comparison, a 5-minute default replay window, and multiple `v1` tokens
  accepted so a key rotation verifies against either secret. The tests pin the
  same cross-implementation vectors the Python SDK does, produced by the
  TypeScript signer.

### Notes

- Cancelling a scheduled email is `POST /v1/emails/:id/cancel`. There is no
  `DELETE` on emails.
- `to`, `cc` and `bcc` are capped at 50 recipients **combined** — the provider
  refuses more, so a larger send would be accepted and then die downstream.
- Minimum supported Rust version 1.75, edition 2021.
