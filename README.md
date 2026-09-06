# mailtea-rust

The official Rust SDK for [Mailtea](https://mailtea.app) — a thin, typed wrapper
over the [REST API](https://docs.mailtea.app/docs/api-reference). Async, on
`tokio` + `reqwest`. Rust 1.75+.

## Install

```bash
cargo add mailtea tokio --features tokio/macros,tokio/rt-multi-thread
```

Or in `Cargo.toml`:

```toml
[dependencies]
mailtea = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Usage

```rust
use mailtea::{Mailtea, SendEmail};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reads MAILTEA_API_KEY from the environment.
    let mailtea = Mailtea::from_env()?;

    let sent = mailtea
        .emails
        .send(
            &SendEmail::new(
                "you@yourdomain.com",
                ["recipient@example.com"],
                "Hello from Mailtea",
            )
            .html("<p>Your first email, sent with <strong>Mailtea</strong>.</p>"),
        )
        .await?;

    println!("{}", sent.id);

    let email = mailtea.emails.get(&sent.id).await?;
    println!("{}", email.status.unwrap_or_default());
    Ok(())
}
```

`SendEmail` also works as a struct literal, and every method equally accepts a
`serde_json::json!` payload — handy when you already hold the JSON:

```rust
let sent = mailtea
    .emails
    .send(serde_json::json!({
        "from": "you@yourdomain.com",
        "to": "recipient@example.com",
        "subject": "Hello from Mailtea",
        "html": "<p>Your first email, sent via the API.</p>"
    }))
    .await?;
```

Payload field names are the REST wire names (`reply_to`, `scheduled_at`, …).
`to`, `cc` and `bcc` are capped at **50 recipients combined** — the provider
refuses more.

## Configuration

| Variable | What it does |
| --- | --- |
| `MAILTEA_API_KEY` | The key `Mailtea::from_env()` reads. |
| `MAILTEA_API_BASE_URL` | Points the client at a self-hosted or local Mailtea. Defaults to `https://api.mailtea.app`. |

```rust
use mailtea::Mailtea;

// Key from the environment, base URL from the environment or the default.
let mailtea = Mailtea::from_env()?;

// Or explicitly. A trailing slash on the base URL is stripped.
let mailtea = Mailtea::builder()
    .api_key(std::env::var("MAILTEA_API_KEY")?)
    .base_url("http://127.0.0.1:7787")
    .build()?;
```

Every request goes through a `Transport`, so tests need no network. Implement
it to mock the API, add retries or tracing, or route through your own HTTP
stack:

```rust
use mailtea::{BoxFuture, HttpRequest, HttpResponse, Mailtea, Result, Transport};
use std::collections::HashMap;

#[derive(Debug)]
struct AlwaysAccepts;

impl Transport for AlwaysAccepts {
    fn execute<'a>(&'a self, _request: HttpRequest) -> BoxFuture<'a, Result<HttpResponse>> {
        Box::pin(async {
            Ok(HttpResponse {
                status: 200,
                headers: HashMap::new(),
                body: r#"{"id":"txemail_1"}"#.to_string(),
            })
        })
    }
}

let mailtea = Mailtea::builder()
    .api_key("mt_pat_test")
    .transport(AlwaysAccepts)
    .build()?;
```

`ReqwestTransport::with_client` takes a `reqwest::Client` of your own if you only
want to change the connection pool or proxy. The default carries a 30-second
per-request timeout.

## API

Every method takes `impl Serialize` — a typed request struct, a
`serde_json::json!` literal, or a struct of your own. Pass `()` where you have no
parameters. Responses are typed on the transactional email path and
`serde_json::Value` elsewhere; `Mailtea::request` deserializes any endpoint into
a type of yours.

| Method | Description |
| --- | --- |
| `emails.send(email)` | Send a transactional email → `SentEmail { id }` |
| `emails.send_idempotent(email, key)` | The same send with an `Idempotency-Key`, so a retry cannot deliver twice |
| `emails.batch(emails)` | Send up to 100 emails → `BatchSent { data }` |
| `emails.get(id)` | Retrieve an email and its delivery status → `Email` |
| `emails.list(params)` | List emails → `{"data", "total", "limit", "offset", "has_more"}` |
| `emails.update(id, params)` | Update a scheduled email (currently only `scheduled_at`) |
| `emails.reschedule(id, scheduled_at)` | Convenience wrapper over `update` |
| `emails.cancel(id)` | Cancel a scheduled email (`POST /v1/emails/:id/cancel`) |
| `emails.analytics(params)` | Aggregate transactional metrics over an optional date window |
| `emails.inbound.list(params)` | List received emails in a publication (cursor-paginated) |
| `emails.inbound.get(id)` | Retrieve a received email with body, headers, and attachments |
| `emails.inbound.reply(id, params)` | Reply to a received email (threads by construction) |
| `emails.inbound.attachments.list(id)` | List a received email's attachments (signed download URLs) |
| `emails.inbound.attachments.get(id, attachment_id)` | Retrieve one inbound attachment |
| `contacts.create / upsert / list / get / update / delete` | Manage audience contacts (`upsert` = `create`; the endpoint upserts) |
| `posts.create(params)` | Create a newsletter post (draft, or `send: true`) |
| `posts.send(id, params)` | Send a draft post to the audience, now or scheduled |
| `posts.send_test(id, params)` | Send a `[TEST]` copy of a post → `{"sent_to", "failed_to"}` |
| `posts.list / get / update / delete` | Manage posts |
| `segments.create / list / get / update / delete` | Manage audience segments |
| `topics.create / list / get / update / delete` | Manage topic definitions (`visibility: "public"` → shown on the reader preference page) |
| `senders.create / list / get / update / delete` | Manage named From identities (`email` immutable) |
| `assets.upload / list / delete` | The publication's image library |
| `templates.create / list / get / update / publish / unpublish / duplicate / delete` | Manage reusable email templates |
| `templates.render(params)` | Render a spec to HTML without saving → `{"html", "text"}` |
| `templates.versions(id, params)` | List a template's design history, newest first (metadata only) |
| `templates.restore_version(id, version, params)` | Put an older design back — a content write, so the template returns to **draft** |
| `suppressions.list / add / remove` | Manage the team-wide do-not-send list |
| `suppressions.export()` | Export the whole suppression list as CSV (raw text) |
| `domains.create / list / get / verify / update / delete` | Manage sending domains (add, read DNS records, verify) |
| `domains.tracking.create / list / verify / delete` | Manage CNAME tracking sub-domains under a domain |
| `webhooks.create / list / get / update / delete` | Manage outbound event subscriptions |
| `contact_properties.create / list / update / delete` | Manage custom contact fields (team-scoped) |
| `api_keys.create / list / revoke` | Manage API keys (`settings:write`) |
| `automations.create / list / get / update / delete` | Manage automation graphs (`steps` + optional `connections`) |
| `automations.validate(params)` | Dry-run a graph → `{"valid", "issues"}` |
| `automations.activate / pause / archive` | Lifecycle (`cancel_runs` defaults **false** on pause, **true** on archive) |
| `automations.versions(id, …)` / `automations.version(id, version, …)` | List stored versions; retrieve one with its graph |
| `automations.metrics(id, params)` | Per-step funnel counts and branch splits (test runs excluded) |
| `automations.test(id, params)` | One test run against a real contact — **sends real, billed email** |
| `automation_runs.list / get / cancel` | Inspect and cancel runs (a run pins the version it started on) |
| `events.send(params)` | Record a custom event → `{"enrolled_automations", "resumed_runs"}` |
| `events.list(params)` | List recorded events (cursor-paginated) |
| `event_definitions.create / list / get / update / delete` | Manage the event catalog (`name` immutable) |

Typed request structs are provided for the payloads worth checking at compile
time: `SendEmail` (with `Tag`, `Attachment`, `TemplateRef`), `CreateContact`,
`UpdateContact`, `CreatePost`, `SendPost`, `SendTestPost`, `CreateTopic` and
`UploadAsset`. Everything else takes a `serde_json::json!` map — the long tail of
filters and graph shapes is not worth a struct that lags the API. Each of those
structs carries an `extra` map flattened into the same JSON object, so a field
the API grows before this crate does still reaches the wire.

`emails.send` also accepts `tags`, custom `headers`, `attachments` and
`scheduled_at`. `Attachment::from_bytes` base64-encodes for you; set a
`content_id` (plus `content_type`) to embed an inline image referenced by `cid:`
in the HTML:

```rust
use mailtea::{Attachment, SendEmail};

mailtea.emails.send(
    &SendEmail::new("you@yourdomain.com", ["recipient@example.com"], "Your receipt")
        .html(r#"<p>Thanks!</p><img src="cid:logo" />"#)
        .tag("category", "receipt")
        .attachment(Attachment::from_bytes("receipt.pdf", pdf_bytes))
        .attachment(
            Attachment::from_bytes("logo.png", logo_bytes)
                .content_type("image/png")
                .content_id("logo"),
        ),
).await?;
```

## Webhooks

Mailtea signs every outbound webhook with
[Standard Webhooks](https://www.standardwebhooks.com/).
`verify_webhook_signature` checks the signature and rejects replays. Pass the
**raw** request body — not re-serialized JSON — and the endpoint's `whsec_…`
signing secret:

```rust
use mailtea::verify_webhook_signature;

let ok = verify_webhook_signature(
    &signing_secret,                    // whsec_… from webhooks.create
    headers["webhook-id"],
    headers["webhook-timestamp"],       // unix seconds, as the header sends it
    &raw_body,                          // exact bytes received
    headers["webhook-signature"],
);
if !ok {
    return unauthorized();
}
```

The comparison is constant-time and the default replay window is 5 minutes;
`verify_webhook_signature_with` takes a `VerifyOptions` to change the tolerance
or inject the clock. `sign_webhook` produces the same header, for faking
deliveries in tests.

## Errors

Every call returns `Result<T, mailtea::Error>`. One struct carries everything a
caller needs:

| Accessor | What it holds |
| --- | --- |
| `status()` | HTTP status. **`0` means the request never reached the API** — nothing was sent. |
| `message()` | The API's own `error` string, or the client-side fault. |
| `code()` | Machine-readable code when the API sends one (`marketing_plan_required`), or `missing_api_key` / `transport_error` / `invalid_response` client-side. |
| `details()` | The `details` array a 400 carries — one entry per field refused. "Validation failed" alone does not say what to change. |
| `request_id()` | The `x-request-id` header. Quote it in a support request. |
| `is_client_error()` | `status() == 0`. |
| `is_retryable()` | 429 and 5xx. Retry with the same `Idempotency-Key`. |

```rust
match mailtea.emails.send(&email).await {
    Ok(sent) => println!("Sent {}", sent.id),
    Err(error) if error.is_client_error() => {
        // DNS, TLS, timeout, or a missing key. Nothing was sent.
        eprintln!("{error}");
    }
    Err(error) => {
        eprintln!("HTTP {}: {}", error.status(), error.message());
        if let Some(details) = error.details() {
            eprintln!("{details}");
        }
    }
}
```

`Error` implements `std::error::Error`, so it works with `?`, `Box<dyn Error>`,
`anyhow` and `thiserror`'s `#[from]`.

## Local development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The suite runs against a mock Mailtea served over a `tokio` `TcpListener`
(`tests/common/mod.rs`), so it needs no API key and makes no network calls.
`tests/endpoint_parity.rs` checks that this crate reaches every endpoint the
other official SDKs do.

Against a local Mailtea:

```bash
export MAILTEA_API_KEY="mt_pat_…"
export MAILTEA_API_BASE_URL="http://127.0.0.1:7787"
cargo run   # from a binary crate that depends on this one
```

The minimum supported Rust version is 1.75. Building on exactly 1.75 needs an
MSRV-aware dependency resolution, because the newest releases of some transitive
dependencies require a newer toolchain:

```bash
CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback cargo generate-lockfile
cargo +1.75 test --locked
```

On current stable, `cargo test` needs none of that.

## License

MIT — see [LICENSE](LICENSE).
