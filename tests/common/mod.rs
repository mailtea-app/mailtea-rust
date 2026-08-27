//! A tiny stand-in for the Mailtea API, so this crate's tests run with no
//! credentials and no network. It records every request it receives, which is
//! what the assertions read.
//!
//! Its behaviour is ported from `examples/.shared/node/mock-mailtea.mjs` in the
//! Mailtea monorepo: auth is checked first, every recorded route answers a fixed
//! shape, and anything the API does not define answers 404 — a mock that
//! implements a route the API does not have is worse than no mock, because the
//! tests go green against a lie.

#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub const API_KEY: &str = "mt_pat_testkeytestkeytestkeytestkey00";
pub const EMAIL_ID: &str = "txemail_00000000000000000000000000000000";
pub const REQUEST_ID: &str = "req_00000000000000000000000000000000";

#[derive(Clone, Debug)]
pub struct RecordedRequest {
    pub method: String,
    pub path: String,
    /// The raw query string without its `?`, or `None` when there was none.
    pub query: Option<String>,
    pub authorization: Option<String>,
    pub user_agent: Option<String>,
    pub idempotency_key: Option<String>,
    /// `Value::Null` when the request carried no JSON body.
    pub body: Value,
}

impl RecordedRequest {
    /// One query parameter, decoded.
    pub fn param(&self, name: &str) -> Option<String> {
        let query = self.query.as_deref()?;
        query.split('&').find_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            (percent_decode(key) == name).then(|| percent_decode(value))
        })
    }
}

pub struct MockMailtea {
    pub url: String,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
}

impl MockMailtea {
    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().unwrap().clone()
    }

    /// The most recent request, which is what most assertions want.
    pub fn last(&self) -> RecordedRequest {
        self.requests()
            .pop()
            .expect("the mock received no requests")
    }
}

/// Binds an ephemeral port and serves until the test process exits.
pub async fn start() -> MockMailtea {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("could not bind the mock server");
    let url = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));

    let recorded = Arc::clone(&requests);
    tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            let recorded = Arc::clone(&recorded);
            tokio::spawn(async move { serve(socket, recorded).await });
        }
    });

    MockMailtea { url, requests }
}

/// A client pointed at a freshly started mock.
pub async fn client() -> (MockMailtea, mailtea::Mailtea) {
    let mock = start().await;
    let client = mailtea::Mailtea::builder()
        .api_key(API_KEY)
        .base_url(mock.url.clone())
        .build()
        .expect("an explicit key and base URL build");
    (mock, client)
}

async fn serve(mut socket: TcpStream, recorded: Arc<Mutex<Vec<RecordedRequest>>>) {
    let Some(request) = read_request(&mut socket).await else {
        return;
    };

    let authorized = request
        .authorization
        .as_deref()
        .is_some_and(|value| value.starts_with("Bearer "));
    recorded.lock().unwrap().push(request.clone());

    // Auth is checked first, the same way the real API does it — a client that
    // forgets the key should fail its test, not silently "send".
    let (status, content_type, body) = if !authorized {
        (
            401,
            "application/json",
            json!({ "error": "Unauthorized" }).to_string(),
        )
    } else if request.method == "GET" && request.path == "/v1/suppressions/export" {
        // The one endpoint that does not answer JSON.
        (
            200,
            "text/csv",
            "email,reason,source,created_at\nblocked@example.com,bounce,ses,2026-01-01T00:00:00.000Z\n"
                .to_string(),
        )
    } else {
        let (status, payload) = route(&request);
        (status, "application/json", payload.to_string())
    };

    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\nx-request-id: {REQUEST_ID}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = socket.write_all(response.as_bytes()).await;
    let _ = socket.shutdown().await;
}

fn list_of(object: &str) -> Value {
    json!({
        "object": "list",
        "data": [{ "object": object, "id": format!("{object}_1") }],
        "total": 1,
        "limit": 20,
        "offset": 0,
        "has_more": false
    })
}

fn object(kind: &str, id: &str) -> Value {
    json!({ "object": kind, "id": id })
}

/// The route table. Specific paths come before the `:id` ones, because match
/// arms are tried in order.
fn route(request: &RecordedRequest) -> (u16, Value) {
    let method = request.method.as_str();
    let segments: Vec<&str> = request
        .path
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();

    match (method, segments.as_slice()) {
        // ---- emails -------------------------------------------------------
        ("POST", ["v1", "emails"]) => {
            // The real API validates before it sends; so does this, because the
            // error path is half of what the tests are checking.
            let from = request.body.get("from").and_then(Value::as_str);
            let sender_id = request.body.get("sender_id");
            if from.unwrap_or("").is_empty() && sender_id.is_none() {
                // Shaped like the real 400: `error` plus a `details` array
                // naming each field it refused.
                (
                    400,
                    json!({
                        "error": "Validation failed",
                        "details": [{
                            "code": "too_small",
                            "path": ["from"],
                            "message": "String must contain at least 1 character(s)"
                        }]
                    }),
                )
            } else {
                (200, json!({ "id": EMAIL_ID }))
            }
        }
        ("POST", ["v1", "emails", "batch"]) => {
            let count = request.body.as_array().map_or(0, Vec::len);
            (
                200,
                json!({
                    "data": (0..count)
                        .map(|index| json!({ "id": format!("txemail_{index:032}") }))
                        .collect::<Vec<_>>()
                }),
            )
        }
        ("GET", ["v1", "emails", "analytics"]) => (
            200,
            json!({
                "object": "email_analytics",
                "total": 3,
                "delivered": 2,
                "bounced": 1,
                "from_date": "2026-07-28T00:00:00.000Z"
            }),
        ),
        ("GET", ["v1", "emails"]) => (200, list_of("email")),
        // Inbound sits under /v1/emails, so it must be matched before `:id`.
        ("GET", ["v1", "emails", "inbound"]) => (200, list_of("inbound_email")),
        ("GET", ["v1", "emails", "inbound", id]) => (200, object("inbound_email", id)),
        ("POST", ["v1", "emails", "inbound", _id, "reply"]) => {
            (200, json!({ "id": EMAIL_ID, "status": "queued" }))
        }
        ("GET", ["v1", "emails", "inbound", _id, "attachments"]) => (
            200,
            json!({
                "object": "list",
                "data": [{
                    "object": "inbound_attachment",
                    "id": "att_1",
                    "download_url": "https://example.invalid/signed"
                }]
            }),
        ),
        ("GET", ["v1", "emails", "inbound", _id, "attachments", attachment_id]) => (
            200,
            json!({
                "object": "inbound_attachment",
                "id": attachment_id,
                "download_url": "https://example.invalid/signed"
            }),
        ),
        ("GET", ["v1", "emails", id]) => (
            200,
            json!({
                "object": "email",
                "id": id,
                "to": "reader@yourdomain.com",
                "subject": "Mock email",
                "last_event": "delivered",
                "created_at": "2026-01-01T00:00:00.000Z"
            }),
        ),
        ("PATCH", ["v1", "emails", id]) => (
            200,
            json!({
                "object": "email",
                "id": id,
                "last_event": "scheduled",
                "scheduled_at": request.body.get("scheduled_at").cloned().unwrap_or(Value::Null)
            }),
        ),
        // Cancel is POST /v1/emails/:id/cancel. There is no DELETE on emails —
        // the real API does not define one.
        ("POST", ["v1", "emails", id, "cancel"]) => (
            200,
            json!({ "object": "email", "id": id, "last_event": "canceled" }),
        ),

        // ---- contacts -----------------------------------------------------
        ("POST", ["v1", "contacts"]) => (200, object("contact", "con_1")),
        ("GET", ["v1", "contacts"]) => (200, list_of("contact")),
        ("GET", ["v1", "contacts", id]) => (200, object("contact", id)),
        ("PATCH", ["v1", "contacts", id]) => (200, object("contact", id)),
        ("DELETE", ["v1", "contacts", id]) => (200, json!({ "deleted": true, "id": id })),

        // ---- segments -----------------------------------------------------
        ("POST", ["v1", "segments"]) => (200, object("segment", "seg_1")),
        ("GET", ["v1", "segments"]) => (200, list_of("segment")),
        ("GET", ["v1", "segments", id]) => (200, object("segment", id)),
        ("PATCH", ["v1", "segments", id]) => (200, object("segment", id)),
        ("DELETE", ["v1", "segments", id]) => (200, json!({ "deleted": true, "id": id })),

        // ---- topics -------------------------------------------------------
        ("POST", ["v1", "topics"]) => (
            200,
            json!({
                "object": "topic",
                "id": "top_1",
                "name": request.body.get("name").cloned().unwrap_or(Value::Null)
            }),
        ),
        ("GET", ["v1", "topics"]) => (200, list_of("topic")),
        ("GET", ["v1", "topics", id]) => (200, object("topic", id)),
        ("PATCH", ["v1", "topics", id]) => (200, object("topic", id)),
        ("DELETE", ["v1", "topics", id]) => (200, json!({ "deleted": true, "id": id })),

        // ---- posts --------------------------------------------------------
        ("POST", ["v1", "posts"]) => (200, json!({ "id": "post_1" })),
        ("GET", ["v1", "posts"]) => (200, json!({ "data": [{ "id": "post_1" }], "total": 1 })),
        ("GET", ["v1", "posts", id]) => (200, object("post", id)),
        ("PATCH", ["v1", "posts", id]) => (200, object("post", id)),
        ("DELETE", ["v1", "posts", id]) => (200, json!({ "deleted": true, "id": id })),
        ("POST", ["v1", "posts", id, "send"]) => (200, json!({ "id": id, "status": "sending" })),
        ("POST", ["v1", "posts", _id, "test"]) => (
            200,
            json!({ "sent_to": ["you@example.com"], "failed_to": [] }),
        ),

        // ---- senders ------------------------------------------------------
        ("POST", ["v1", "senders"]) => (200, object("sender", "snd_1")),
        ("GET", ["v1", "senders"]) => (200, list_of("sender")),
        ("GET", ["v1", "senders", id]) => (200, object("sender", id)),
        ("PATCH", ["v1", "senders", id]) => (200, object("sender", id)),
        ("DELETE", ["v1", "senders", id]) => (200, json!({ "deleted": true, "id": id })),

        // ---- assets -------------------------------------------------------
        ("POST", ["v1", "assets"]) => (
            200,
            json!({
                "object": "asset",
                "id": "ast_1",
                "url": "https://assets.example.invalid/ast_1.png"
            }),
        ),
        ("GET", ["v1", "assets"]) => (200, list_of("asset")),
        ("DELETE", ["v1", "assets", id]) => (200, json!({ "deleted": true, "id": id })),

        // ---- suppressions -------------------------------------------------
        ("GET", ["v1", "suppressions"]) => (200, list_of("suppression")),
        ("POST", ["v1", "suppressions"]) => (200, json!({ "added": 1 })),
        ("DELETE", ["v1", "suppressions"]) => (200, json!({ "removed": 1 })),

        // ---- templates ----------------------------------------------------
        ("POST", ["v1", "templates", "render"]) => (
            200,
            json!({ "html": "<p>Rendered</p>", "text": "Rendered" }),
        ),
        ("POST", ["v1", "templates"]) => (200, object("template", "tpl_1")),
        ("GET", ["v1", "templates"]) => (200, list_of("template")),
        ("GET", ["v1", "templates", id]) => (200, object("template", id)),
        ("PATCH", ["v1", "templates", id]) => (200, object("template", id)),
        ("DELETE", ["v1", "templates", id]) => (200, json!({ "deleted": true, "id": id })),
        ("POST", ["v1", "templates", id, "publish"]) => (
            200,
            json!({ "object": "template", "id": id, "status": "published" }),
        ),
        ("POST", ["v1", "templates", id, "unpublish"]) => (
            200,
            json!({ "object": "template", "id": id, "status": "draft" }),
        ),
        ("GET", ["v1", "templates", _id, "versions"]) => (
            200,
            json!({ "object": "list", "data": [{ "version": 2, "is_current": true }] }),
        ),
        ("POST", ["v1", "templates", _id, "versions", version, "restore"]) => (
            200,
            json!({
                "restored": true,
                "restored_from_version": version.parse::<u32>().unwrap_or(0),
                "unpublished": true
            }),
        ),
        ("POST", ["v1", "templates", _id, "duplicate"]) => (200, object("template", "tpl_2")),

        // ---- domains ------------------------------------------------------
        ("POST", ["v1", "domains"]) => (
            200,
            json!({ "object": "domain", "id": "dom_1", "records": [] }),
        ),
        ("GET", ["v1", "domains"]) => (200, list_of("domain")),
        ("GET", ["v1", "domains", id]) => (200, object("domain", id)),
        ("PATCH", ["v1", "domains", id]) => (200, object("domain", id)),
        ("DELETE", ["v1", "domains", id]) => (200, json!({ "deleted": true, "id": id })),
        ("POST", ["v1", "domains", id, "verify"]) => (
            200,
            json!({ "object": "domain", "id": id, "status": "verified" }),
        ),
        ("POST", ["v1", "domains", _id, "tracking-domains"]) => (
            200,
            json!({ "object": "tracking_domain", "id": "trk_1", "records": [] }),
        ),
        ("GET", ["v1", "domains", _id, "tracking-domains"]) => (200, list_of("tracking_domain")),
        ("POST", ["v1", "domains", _id, "tracking-domains", tracking_id, "verify"]) => (
            200,
            json!({ "object": "tracking_domain", "id": tracking_id, "status": "verified" }),
        ),
        ("DELETE", ["v1", "domains", _id, "tracking-domains", tracking_id]) => {
            (200, json!({ "deleted": true, "id": tracking_id }))
        }

        // ---- webhooks -----------------------------------------------------
        ("POST", ["v1", "webhooks", "endpoints"]) => (
            200,
            json!({
                "object": "webhook_endpoint",
                "id": "whe_1",
                "signing_secret": "whsec_dGVzdHNpZ25pbmdrZXlub3RhcmVhbHNlY3JldA=="
            }),
        ),
        ("GET", ["v1", "webhooks", "endpoints"]) => (200, list_of("webhook_endpoint")),
        ("GET", ["v1", "webhooks", "endpoints", id]) => (200, object("webhook_endpoint", id)),
        ("PATCH", ["v1", "webhooks", "endpoints", id]) => (200, object("webhook_endpoint", id)),
        ("DELETE", ["v1", "webhooks", "endpoints", id]) => {
            (200, json!({ "deleted": true, "id": id }))
        }

        // ---- contact properties -------------------------------------------
        ("POST", ["v1", "contact-properties"]) => (200, object("contact_property", "cpr_1")),
        ("GET", ["v1", "contact-properties"]) => (200, list_of("contact_property")),
        ("PATCH", ["v1", "contact-properties", id]) => (200, object("contact_property", id)),
        ("DELETE", ["v1", "contact-properties", id]) => (200, json!({ "deleted": true, "id": id })),

        // ---- api keys -----------------------------------------------------
        ("POST", ["v1", "api-keys"]) => (
            200,
            json!({ "object": "api_key", "id": "key_1", "token": "mt_svc_returned_once" }),
        ),
        ("GET", ["v1", "api-keys"]) => (200, list_of("api_key")),
        ("DELETE", ["v1", "api-keys", id]) => (200, json!({ "deleted": true, "id": id })),

        // ---- automations --------------------------------------------------
        ("POST", ["v1", "automations", "validate"]) => (
            200,
            json!({ "object": "automation_validation", "valid": true, "issues": [] }),
        ),
        ("POST", ["v1", "automations"]) => (
            200,
            json!({ "object": "automation", "id": "aut_1", "status": "draft" }),
        ),
        ("GET", ["v1", "automations"]) => (200, list_of("automation")),
        ("GET", ["v1", "automations", id]) => (200, object("automation", id)),
        ("PATCH", ["v1", "automations", id]) => (200, object("automation", id)),
        ("DELETE", ["v1", "automations", id]) => (200, json!({ "deleted": true, "id": id })),
        ("POST", ["v1", "automations", id, "activate"]) => (
            200,
            json!({ "object": "automation", "id": id, "status": "active" }),
        ),
        ("POST", ["v1", "automations", id, "pause"]) => (
            200,
            json!({
                "object": "automation",
                "id": id,
                "status": "paused",
                "canceled_runs": 0
            }),
        ),
        ("POST", ["v1", "automations", id, "archive"]) => (
            200,
            json!({
                "object": "automation",
                "id": id,
                "status": "archived",
                "canceled_runs": 2
            }),
        ),
        ("GET", ["v1", "automations", _id, "versions"]) => (200, list_of("automation_version")),
        ("GET", ["v1", "automations", _id, "versions", version]) => (
            200,
            json!({
                "object": "automation_version",
                "version": version.parse::<u32>().unwrap_or(0),
                "steps": [],
                "connections": []
            }),
        ),
        ("GET", ["v1", "automations", _id, "metrics"]) => (
            200,
            json!({ "object": "automation_metrics", "excludes_test_runs": true, "steps": [] }),
        ),
        ("POST", ["v1", "automations", _id, "test"]) => (
            202,
            json!({ "object": "automation_run", "id": "run_1", "is_test": true }),
        ),
        ("GET", ["v1", "automations", _id, "runs"]) => (200, list_of("automation_run")),
        ("GET", ["v1", "automations", _id, "runs", run_id]) => {
            (200, object("automation_run", run_id))
        }
        ("POST", ["v1", "automations", _id, "runs", run_id, "cancel"]) => (
            200,
            json!({ "object": "automation_run", "id": run_id, "status": "canceled" }),
        ),

        // ---- events -------------------------------------------------------
        ("POST", ["v1", "events"]) => (
            202,
            json!({
                "object": "event",
                "id": "evt_1",
                "enrolled_automations": 1,
                "resumed_runs": 0
            }),
        ),
        ("GET", ["v1", "events"]) => (200, list_of("event")),
        ("POST", ["v1", "event-definitions"]) => (200, object("event_definition", "evd_1")),
        ("GET", ["v1", "event-definitions"]) => (200, list_of("event_definition")),
        ("GET", ["v1", "event-definitions", id]) => (200, object("event_definition", id)),
        ("PATCH", ["v1", "event-definitions", id]) => (200, object("event_definition", id)),
        ("DELETE", ["v1", "event-definitions", id]) => (200, json!({ "deleted": true, "id": id })),

        _ => (404, json!({ "error": "Not Found", "path": request.path })),
    }
}

async fn read_request(socket: &mut TcpStream) -> Option<RecordedRequest> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];

    let head_end = loop {
        if let Some(index) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
        match socket.read(&mut chunk).await {
            Ok(0) | Err(_) => return None,
            Ok(read) => buffer.extend_from_slice(&chunk[..read]),
        }
    };

    let head = String::from_utf8_lossy(&buffer[..head_end]).into_owned();
    let mut request_line = head.lines().next()?.split_whitespace();
    let method = request_line.next()?.to_string();
    let target = request_line.next()?;
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path.to_string(), Some(query.to_string())),
        None => (target.to_string(), None),
    };
    let header = |name: &str| {
        head.lines().skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim()
                .eq_ignore_ascii_case(name)
                .then(|| value.trim().to_string())
        })
    };

    let length: usize = header("content-length")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    while buffer.len() < head_end + length {
        match socket.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(read) => buffer.extend_from_slice(&chunk[..read]),
        }
    }

    Some(RecordedRequest {
        method,
        path,
        query,
        authorization: header("authorization"),
        user_agent: header("user-agent"),
        idempotency_key: header("idempotency-key"),
        body: serde_json::from_slice(&buffer[head_end..]).unwrap_or(Value::Null),
    })
}

/// Enough percent-decoding to read back what the SDK encoded.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        index += 3;
                    }
                    Err(_) => {
                        out.push(bytes[index]);
                        index += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
