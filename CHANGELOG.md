# Changelog

All notable changes to the `mailtea` Rust crate are documented here.

## 0.5.0 (2026-09-28)

- Breaking: `CreatePost::template_id` seeding now HTML-escapes the `variables`
  you pass, the same as every other send. HTML passed in a `{{key}}` value now
  arrives as visible text, and a value you escaped yourself arrives
  double-escaped. Put `{{{key}}}` in the template where a value is meant to be
  raw HTML. Variables are now filled in both the `{{key}}` and Visual Email
  Designer `{key}` forms. A declared variable you do not pass stays in the
  post with its `fallback_value`, so the broadcast gives each recipient their
  own value or that fallback, and undeclared tokens like
  `{{contact.first_name}}` are left for the broadcast too. The post keeps the
  template's published page style, is wrapped in that page, and has its
  show-if blocks decided per recipient when it is sent. Before, only the
  variables you passed were replaced, raw, and only in `{{key}}` form. It uses
  the template's published version; Mailtea Studio's "Use template" starts
  from the latest saved design instead.
- Changed: `Templates::update` and `Templates::restore_version` no longer move
  a published template back to draft. The template keeps its published status,
  and automations and the API keep sending its published version until
  `Templates::publish` is called again. The template's `from` and `reply_to`
  are part of the published version too, so a new sender or reply-to address
  reaches sends only after the next publish. `Templates::unpublish` is now the
  only way to stop a published template sending, short of deleting it, and it
  drops the stored published version so the next publish starts from the
  current content.
- Added: `has_unpublished_versions` on every returned template `Value`. True
  only when the template is published and its saved content (From, Reply-To and the style profile
  included) differs from the published version.
- Added: `is_published` on each template version entry: true for the one entry
  automations and the API are sending now. `is_current` is now described as
  what it is: the entry that matches the working copy (the saved design being
  edited), not necessarily what is sending. `is_published` is false on every
  entry of a draft, and on a template published before the field existed until
  it is published again.
- Changed: the `unpublished` field on the update and restore replies is kept
  for compatibility and is now always `false`. Check
  `has_unpublished_versions` (or the reply's `message`) instead.
- Changed (API behavior): a template variable's `fallback_value` can no longer
  contain `{` or `}`. Creating a template with one, or changing a fallback
  to one on update, is a 400 ("Fallbacks can't contain { or }."). A value
  the template already stores is accepted unchanged, so a template saved
  before the rule keeps saving. Inline chip fallbacks such as
  `{first_name|Mom & Pop}` now render as written instead of double-escaped.
- Changed (API behavior): saving an active automation is refused only when the
  edit adds an error the live version does not already have. The 422
  `active_graph_invalid` reply's `issues` lists just those new problems.
  Before, any error refused the save, even one the live version already had.
  Starting refuses every error as before, except an `unknown_step_ref` at a
  `config.*` path or a trigger `missing_branch` that the version the
  automation last ran on already had, so pausing and starting an unchanged
  automation keeps working. Issues the last live version already had come back
  with `pre_existing: true`.
- Added (API behavior): issue objects carry `field`, what a rule reads (the
  rule's `field`, or the path in a `{"var": ...}` value, e.g.
  `steps.welcome.opened`) when the issue is about one.
- Changed (API behavior): two issues are the same problem when their code and
  step match, and their `field` or, when there is none, their `path`. Moving
  a rule, by removing a rule beside it or putting it in a group, no longer
  makes a problem the live version already had look new. An error is
  `pre_existing` only if the live version had an error there, not a warning.
- Changed (API behavior): `validate_only` on an active automation answers the
  way the save would. A trigger change is a 422 `trigger_locked_while_active`,
  a change that adds a problem is a 422 `active_graph_invalid` listing only
  the new problems, and otherwise issues come back with `pre_existing` marked
  against the version live now. Before, it returned every issue unmarked.
- Changed (API behavior): changing the trigger (its type or key) of an active
  automation is now refused with 422 `trigger_locked_while_active`. Pause it
  first; draft and paused automations can still change their trigger. Before,
  the change was accepted.
- Added (API behavior): new validation rules. A trigger with nothing after it
  is a `missing_branch` error at `branches.next`. A rule or `{"var": ...}`
  value that reads `steps.<key>.*` for a step that isn't in the automation is
  an `unknown_step_ref` error at that `config.*` path, or a warning when the
  `{"var": ...}` has a `default`. A rule or value that reads
  `event.properties.*` when the automation does not start from an app event is
  the new warning `event_field_without_event_trigger`.

## 0.4.0 (2026-09-15)

- Added: `Email::mode` — `Some("live")` for real mail, `Some("test")` for a
  message sent with a test key (`mt_test_…`), which is validated, recorded and
  webhook-emitting but never delivered. It was already arriving and landing in
  the `#[serde(flatten)] extra` map; this moves it somewhere discoverable. A
  `String` rather than an enum, for the same reason `last_event` is one.
- Added: test mode is reachable through the existing free-form params.
  `api_keys.create(json!({"name": "CI", "mode": "test"}))` mints a test key, and
  `emails.list(json!({"mode": "test"}))` reads test mail. There is no mixed
  view, and a test key is **not** a data sandbox — it reads and writes your real
  contacts, templates, senders and webhooks. Only delivery is simulated.
- Reserved recipients on `test.mailtea.email` force an outcome: `delivered@`,
  `bounced@`, `complained@`, `delayed@`, `failed@`. The first `to` recipient
  decides; anything else is delivered.

## 0.3.0 (2026-09-10)

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
