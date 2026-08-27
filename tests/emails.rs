//! Every assertion here runs against the bundled mock in `common`, so the suite
//! needs no API key and touches no network.

mod common;

use common::{API_KEY, EMAIL_ID};
use mailtea::{Attachment, SendEmail, Tag, TemplateRef};
use serde_json::json;

fn hello() -> SendEmail {
    SendEmail::new("Acme <hello@acme.com>", ["reader@yourdomain.com"], "Hello")
        .html("<p>Sent with Rust.</p>")
        .text("Sent with Rust.")
        .tag("example", "rust")
}

#[tokio::test]
async fn send_posts_the_email_and_returns_its_id() {
    let (mock, mailtea) = common::client().await;

    let sent = mailtea.emails.send(&hello()).await.expect("send failed");

    let request = mock.last();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/emails");
    assert_eq!(
        request.authorization.as_deref(),
        Some(format!("Bearer {API_KEY}").as_str())
    );
    assert_eq!(
        request.user_agent.as_deref(),
        Some(format!("mailtea-rust/{}", mailtea::VERSION).as_str())
    );
    assert_eq!(request.body["from"], "Acme <hello@acme.com>");
    assert_eq!(request.body["to"], json!(["reader@yourdomain.com"]));
    assert_eq!(request.body["subject"], "Hello");
    assert_eq!(request.body["html"], "<p>Sent with Rust.</p>");
    assert_eq!(request.body["text"], "Sent with Rust.");
    assert_eq!(
        request.body["tags"],
        json!([{ "name": "example", "value": "rust" }])
    );
    assert_eq!(sent.id, EMAIL_ID);
}

#[tokio::test]
async fn unset_fields_are_left_out_of_the_body() {
    let (mock, mailtea) = common::client().await;

    mailtea.emails.send(&hello()).await.expect("send failed");

    // An empty `cc` or a null `scheduled_at` on the wire would turn an immediate
    // send into a rejected one, so the optional fields have to disappear.
    let body = mock.last().body;
    for field in [
        "cc",
        "bcc",
        "reply_to",
        "scheduled_at",
        "sender_id",
        "headers",
        "attachments",
        "template",
        "tracking_open",
    ] {
        assert_eq!(body.get(field), None, "{field} should not be serialized");
    }
}

#[tokio::test]
async fn a_json_payload_works_the_same_as_the_typed_one() {
    let (mock, mailtea) = common::client().await;

    let sent = mailtea
        .emails
        .send(json!({
            "from": "Acme <hello@acme.com>",
            "to": "reader@yourdomain.com",
            "subject": "Hello",
            "html": "<p>Hi</p>"
        }))
        .await
        .expect("send failed");

    // A single string recipient is what the API takes too; nothing rewrites it.
    assert_eq!(mock.last().body["to"], "reader@yourdomain.com");
    assert_eq!(sent.id, EMAIL_ID);
}

#[tokio::test]
async fn the_extra_map_reaches_the_same_json_object() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .emails
        .send(&hello().extra("list_unsubscribe", "https://example.invalid/u"))
        .await
        .expect("send failed");

    let body = mock.last().body;
    assert_eq!(body["list_unsubscribe"], "https://example.invalid/u");
    assert_eq!(body["subject"], "Hello", "named fields survive the flatten");
}

#[tokio::test]
async fn a_scheduled_send_carries_scheduled_at() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .emails
        .send(&hello().scheduled_at("2026-09-01T09:00:00Z"))
        .await
        .expect("send failed");

    assert_eq!(mock.last().body["scheduled_at"], "2026-09-01T09:00:00Z");
}

#[tokio::test]
async fn attachments_and_headers_and_templates_serialize() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .emails
        .send(
            &SendEmail::from_sender("snd_1", ["reader@yourdomain.com"], "Receipt")
                .template(TemplateRef::new("tpl_1").variable("name", "Ada"))
                .header("X-Entity-Ref", "order-1138")
                .attachment(Attachment::from_bytes("receipt.txt", b"thanks"))
                .attachment(
                    Attachment::from_base64("logo.png", "aGk=")
                        .content_type("image/png")
                        .content_id("logo"),
                ),
        )
        .await
        .expect("send failed");

    let body = mock.last().body;
    assert_eq!(body["sender_id"], "snd_1");
    assert_eq!(
        body["template"],
        json!({ "id": "tpl_1", "variables": { "name": "Ada" } })
    );
    assert_eq!(body["headers"], json!({ "X-Entity-Ref": "order-1138" }));
    // `from_bytes` base64-encodes for you: "thanks" -> "dGhhbmtz".
    assert_eq!(body["attachments"][0]["filename"], "receipt.txt");
    assert_eq!(body["attachments"][0]["content"], "dGhhbmtz");
    assert_eq!(body["attachments"][0].get("content_id"), None);
    assert_eq!(body["attachments"][1]["content_id"], "logo");
    assert_eq!(body["attachments"][1]["content_type"], "image/png");
}

#[tokio::test]
async fn batch_posts_an_array_and_returns_one_id_each() {
    let (mock, mailtea) = common::client().await;

    let sent = mailtea
        .emails
        .batch([
            &hello(),
            &SendEmail::new("Acme <hello@acme.com>", ["second@yourdomain.com"], "Two"),
        ])
        .await
        .expect("batch failed");

    let request = mock.last();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/emails/batch");
    assert!(request.body.is_array());
    assert_eq!(request.body[1]["subject"], "Two");
    assert_eq!(sent.data.len(), 2);
    assert_eq!(sent.data[0].id, "txemail_00000000000000000000000000000000");
}

#[tokio::test]
async fn get_reads_the_status_back() {
    let (mock, mailtea) = common::client().await;

    let email = mailtea.emails.get(EMAIL_ID).await.expect("get failed");

    let request = mock.last();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, format!("/v1/emails/{EMAIL_ID}"));
    assert_eq!(email.id, EMAIL_ID);
    assert_eq!(email.last_event.as_deref(), Some("delivered"));
    // `status` is the friendly alias the API does not always send.
    assert_eq!(email.status.as_deref(), Some("delivered"));
    assert_eq!(email.subject.as_deref(), Some("Mock email"));
    assert_eq!(email.to, Some(json!("reader@yourdomain.com")));
}

#[tokio::test]
async fn list_and_analytics_render_their_filters_as_a_query() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .emails
        .list(json!({ "status": "delivered", "limit": 5, "offset": null }))
        .await
        .expect("list failed");
    let request = mock.last();
    assert_eq!(request.path, "/v1/emails");
    assert_eq!(request.param("status").as_deref(), Some("delivered"));
    assert_eq!(request.param("limit").as_deref(), Some("5"));
    assert_eq!(request.param("offset"), None, "nulls are dropped");

    let analytics = mailtea
        .emails
        .analytics(json!({ "from_date": "2026-07-01T00:00:00Z" }))
        .await
        .expect("analytics failed");
    let request = mock.last();
    assert_eq!(request.path, "/v1/emails/analytics");
    assert_eq!(
        request.param("from_date").as_deref(),
        Some("2026-07-01T00:00:00Z")
    );
    assert_eq!(analytics["total"], 3);
}

#[tokio::test]
async fn update_and_reschedule_patch_the_email() {
    let (mock, mailtea) = common::client().await;

    let rescheduled = mailtea
        .emails
        .reschedule(EMAIL_ID, "2026-09-01T09:00:00Z")
        .await
        .expect("reschedule failed");

    let request = mock.last();
    assert_eq!(request.method, "PATCH");
    assert_eq!(request.path, format!("/v1/emails/{EMAIL_ID}"));
    assert_eq!(request.body["scheduled_at"], "2026-09-01T09:00:00Z");
    assert_eq!(
        rescheduled.scheduled_at.as_deref(),
        Some("2026-09-01T09:00:00Z")
    );
}

#[tokio::test]
async fn cancel_hits_the_cancel_route() {
    let (mock, mailtea) = common::client().await;

    let canceled = mailtea
        .emails
        .cancel(EMAIL_ID)
        .await
        .expect("cancel failed");

    let request = mock.last();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, format!("/v1/emails/{EMAIL_ID}/cancel"));
    assert_eq!(canceled.id, EMAIL_ID);
    assert_eq!(canceled.last_event.as_deref(), Some("canceled"));
}

#[tokio::test]
async fn an_idempotency_key_rides_along_as_a_header() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .emails
        .send_idempotent(&hello(), Some("order-1138"))
        .await
        .expect("send failed");

    let request = mock.last();
    assert_eq!(request.idempotency_key.as_deref(), Some("order-1138"));
    // A header, not a body field — sending it as one would be a rejected send.
    assert_eq!(request.body.get("idempotency_key"), None);
}

#[tokio::test]
async fn a_plain_send_sets_no_idempotency_key() {
    let (mock, mailtea) = common::client().await;

    mailtea.emails.send(&hello()).await.expect("send failed");

    assert_eq!(mock.last().idempotency_key, None);
}

#[tokio::test]
async fn inbound_reaches_its_own_routes() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .emails
        .inbound
        .list(json!({ "publication_id": "pub_1", "limit": 10 }))
        .await
        .expect("inbound list failed");
    assert_eq!(mock.last().path, "/v1/emails/inbound");
    assert_eq!(
        mock.last().param("publication_id").as_deref(),
        Some("pub_1")
    );

    let received = mailtea
        .emails
        .inbound
        .get("in_1")
        .await
        .expect("inbound get failed");
    assert_eq!(mock.last().path, "/v1/emails/inbound/in_1");
    assert_eq!(received["id"], "in_1");

    let reply = mailtea
        .emails
        .inbound
        .reply("in_1", json!({ "html": "<p>Thanks</p>" }))
        .await
        .expect("inbound reply failed");
    let request = mock.last();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/emails/inbound/in_1/reply");
    assert_eq!(request.body["html"], "<p>Thanks</p>");
    assert_eq!(reply["status"], "queued");

    mailtea
        .emails
        .inbound
        .attachments
        .list("in_1")
        .await
        .expect("attachments list failed");
    assert_eq!(mock.last().path, "/v1/emails/inbound/in_1/attachments");

    let attachment = mailtea
        .emails
        .inbound
        .attachments
        .get("in_1", "att_1")
        .await
        .expect("attachment get failed");
    assert_eq!(
        mock.last().path,
        "/v1/emails/inbound/in_1/attachments/att_1"
    );
    assert_eq!(attachment["id"], "att_1");
}

#[tokio::test]
async fn every_request_is_recorded_in_order() {
    let (mock, mailtea) = common::client().await;

    let sent = mailtea.emails.send(&hello()).await.expect("send failed");
    mailtea.emails.get(&sent.id).await.expect("get failed");
    mailtea
        .emails
        .cancel(&sent.id)
        .await
        .expect("cancel failed");

    let routes: Vec<(String, String)> = mock
        .requests()
        .into_iter()
        .map(|request| (request.method, request.path))
        .collect();
    assert_eq!(
        routes,
        vec![
            ("POST".to_string(), "/v1/emails".to_string()),
            ("GET".to_string(), format!("/v1/emails/{}", sent.id)),
            ("POST".to_string(), format!("/v1/emails/{}/cancel", sent.id)),
        ]
    );
}

#[tokio::test]
async fn an_id_cannot_walk_out_of_its_path_segment() {
    let (mock, mailtea) = common::client().await;

    // Interpolated raw, this would call `GET /v1/domains` instead.
    let _ = mailtea.emails.get("../domains").await;

    assert_eq!(mock.last().path, "/v1/emails/..%2Fdomains");
}

#[tokio::test]
async fn a_base_url_with_a_trailing_slash_still_resolves() {
    let mock = common::start().await;
    let mailtea = mailtea::Mailtea::builder()
        .api_key(API_KEY)
        .base_url(format!("{}/", mock.url))
        .build()
        .expect("build failed");

    mailtea.emails.send(&hello()).await.expect("send failed");

    assert_eq!(mock.last().path, "/v1/emails");
}

#[tokio::test]
async fn tags_can_be_built_by_hand_too() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .emails
        .send(&SendEmail {
            from: Some("Acme <hello@acme.com>".to_string()),
            to: vec!["reader@yourdomain.com".to_string()],
            subject: Some("Struct literal".to_string()),
            tags: vec![Tag::new("category", "receipt")],
            ..Default::default()
        })
        .await
        .expect("send failed");

    assert_eq!(mock.last().body["tags"][0]["name"], "category");
}
