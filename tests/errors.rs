//! What a failed call carries. The typed error is the whole point of an SDK
//! over a raw `fetch`: status, the API's own words, the machine-readable code,
//! the fields a 400 named, and the request id support will ask for.

mod common;

use std::collections::HashMap;

use mailtea::{BoxFuture, HttpRequest, HttpResponse, Mailtea, Result, SendEmail, Transport};
use serde_json::json;

/// A transport that answers the same thing every time — the simplest way to
/// pin an error shape without a server, and a demonstration of the seam.
#[derive(Debug)]
struct Canned {
    status: u16,
    body: String,
    request_id: Option<&'static str>,
}

impl Transport for Canned {
    fn execute<'a>(&'a self, _request: HttpRequest) -> BoxFuture<'a, Result<HttpResponse>> {
        Box::pin(async move {
            let mut headers = HashMap::new();
            if let Some(request_id) = self.request_id {
                headers.insert("x-request-id".to_string(), request_id.to_string());
            }
            Ok(HttpResponse {
                status: self.status,
                headers,
                body: self.body.clone(),
            })
        })
    }
}

fn client_answering(
    status: u16,
    body: serde_json::Value,
    request_id: Option<&'static str>,
) -> Mailtea {
    Mailtea::builder()
        .api_key("mt_pat_test")
        .transport(Canned {
            status,
            body: body.to_string(),
            request_id,
        })
        .build()
        .expect("build failed")
}

fn hello() -> SendEmail {
    SendEmail::new("Acme <hello@acme.com>", ["reader@yourdomain.com"], "Hello")
}

#[tokio::test]
async fn a_rejected_send_surfaces_status_message_details_and_request_id() {
    let (mock, mailtea) = common::client().await;

    let error = mailtea
        .emails
        .send(json!({ "from": "", "to": "reader@yourdomain.com", "subject": "Hi" }))
        .await
        .expect_err("a send with no from address should fail");

    assert_eq!(error.status(), 400);
    assert_eq!(error.message(), "Validation failed");
    // "Validation failed" on its own does not say what to change — `details`
    // names the field.
    assert_eq!(error.details().unwrap()[0]["path"], json!(["from"]));
    assert_eq!(error.request_id(), Some(common::REQUEST_ID));
    assert!(!error.is_client_error());
    assert!(!error.is_retryable());
    assert_eq!(mock.last().path, "/v1/emails");
}

#[tokio::test]
async fn a_machine_readable_code_survives_a_copy_change_to_the_message() {
    let mailtea = client_answering(
        402,
        json!({
            "error": "This endpoint needs a marketing plan.",
            "code": "marketing_plan_required"
        }),
        Some("req_402"),
    );

    let error = mailtea
        .emails
        .send(&hello())
        .await
        .expect_err("402 expected");

    assert_eq!(error.status(), 402);
    assert_eq!(error.code(), Some("marketing_plan_required"));
    assert_eq!(error.request_id(), Some("req_402"));
    assert!(error.to_string().contains("marketing_plan_required"));
}

#[tokio::test]
async fn a_5xx_is_retryable_and_a_4xx_is_not() {
    let server_error = client_answering(503, json!({ "error": "Service unavailable" }), None)
        .emails
        .send(&hello())
        .await
        .expect_err("503 expected");
    assert!(server_error.is_retryable());

    let rate_limited = client_answering(429, json!({ "error": "Too many requests" }), None)
        .emails
        .send(&hello())
        .await
        .expect_err("429 expected");
    assert!(rate_limited.is_retryable());

    let forbidden = client_answering(403, json!({ "error": "Insufficient permissions" }), None)
        .emails
        .send(&hello())
        .await
        .expect_err("403 expected");
    assert!(!forbidden.is_retryable());
}

#[tokio::test]
async fn a_non_json_error_body_keeps_the_status_line() {
    // A proxy's HTML 502 is not JSON. Dropping it would leave an empty message.
    let mailtea = Mailtea::builder()
        .api_key("mt_pat_test")
        .transport(Canned {
            status: 502,
            body: "<html><body>Bad Gateway</body></html>".to_string(),
            request_id: None,
        })
        .build()
        .expect("build failed");

    let error = mailtea
        .emails
        .send(&hello())
        .await
        .expect_err("502 expected");

    assert_eq!(error.status(), 502);
    assert_eq!(error.message(), "HTTP 502");
    assert_eq!(error.code(), None);
    assert_eq!(error.details(), None);
}

#[tokio::test]
async fn a_send_that_never_lands_is_a_client_error() {
    // Port 1 is not listening, which is the closest thing to "the network is
    // down" a test can arrange without a network.
    let mailtea = Mailtea::builder()
        .api_key("mt_pat_test")
        .base_url("http://127.0.0.1:1")
        .build()
        .expect("build failed");

    let error = mailtea
        .emails
        .send(&hello())
        .await
        .expect_err("a send to a closed port should fail");

    // Status 0 means the request never reached the API, so nothing was sent.
    assert_eq!(error.status(), 0);
    assert!(error.is_client_error());
    assert_eq!(error.code(), Some("transport_error"));
    assert!(std::error::Error::source(&error).is_some());
}

#[tokio::test]
async fn a_2xx_body_of_the_wrong_shape_is_a_client_error() {
    let mailtea = client_answering(200, json!({ "unexpected": true }), None);

    let error = mailtea
        .emails
        .send(&hello())
        .await
        .expect_err("a body with no id cannot become a SentEmail");

    assert_eq!(error.status(), 0);
    assert_eq!(error.code(), Some("invalid_response"));
}

#[tokio::test]
async fn a_request_with_no_bearer_token_is_refused_by_the_api() {
    // The mock checks auth first, the same way the real API does. Nothing in
    // the client can produce this, which is exactly why the mock enforces it:
    // an SDK that stopped sending the header would fail here rather than
    // silently "send".
    let mock = common::start().await;
    let response = reqwest::Client::new()
        .post(format!("{}/v1/emails", mock.url))
        .header("content-type", "application/json")
        .body(json!({ "from": "a@b.co" }).to_string())
        .send()
        .await
        .expect("request failed");

    assert_eq!(response.status(), 401);
}
