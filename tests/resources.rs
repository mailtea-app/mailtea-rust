//! One test per resource: the method, the path, the auth header, the body or
//! query it sends, and the field it reads back.

mod common;

use mailtea::{CreateContact, CreatePost, CreateTopic, SendTestPost, UploadAsset};
use serde_json::json;

#[tokio::test]
async fn contacts() {
    let (mock, mailtea) = common::client().await;

    let created = mailtea
        .contacts
        .create(&CreateContact::new("pub_1", "reader@example.com").status("active"))
        .await
        .expect("create failed");
    let request = mock.last();
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("POST", "/v1/contacts")
    );
    assert!(request
        .authorization
        .as_deref()
        .is_some_and(|value| value.starts_with("Bearer ")));
    assert_eq!(request.body["publication_id"], "pub_1");
    assert_eq!(request.body["email"], "reader@example.com");
    assert_eq!(created["id"], "con_1");

    // `upsert` is the same call under the name of what the endpoint does.
    mailtea
        .contacts
        .upsert(json!({ "publication_id": "pub_1", "email": "reader@example.com" }))
        .await
        .expect("upsert failed");
    assert_eq!(mock.last().path, "/v1/contacts");

    mailtea
        .contacts
        .list(json!({ "publication_id": "pub_1", "limit": 50 }))
        .await
        .expect("list failed");
    assert_eq!(
        mock.last().param("publication_id").as_deref(),
        Some("pub_1")
    );

    mailtea
        .contacts
        .get("reader@example.com", json!({ "publication_id": "pub_1" }))
        .await
        .expect("get failed");
    // An email address in a path segment is escaped, `@` included.
    assert_eq!(mock.last().path, "/v1/contacts/reader%40example.com");

    mailtea
        .contacts
        .update(
            "con_1",
            json!({ "publication_id": "pub_1", "status": "unsubscribed" }),
        )
        .await
        .expect("update failed");
    let request = mock.last();
    assert_eq!(request.method, "PATCH");
    // `publication_id` rides in BOTH the query and the body here.
    assert_eq!(request.param("publication_id").as_deref(), Some("pub_1"));
    assert_eq!(request.body["publication_id"], "pub_1");
    assert_eq!(request.body["status"], "unsubscribed");

    mailtea
        .contacts
        .delete("con_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("delete failed");
    let request = mock.last();
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("DELETE", "/v1/contacts/con_1")
    );
    assert_eq!(request.param("publication_id").as_deref(), Some("pub_1"));
}

#[tokio::test]
async fn segments() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .segments
        .create(json!({ "publication_id": "pub_1", "name": "Engaged" }))
        .await
        .expect("create failed");
    assert_eq!(mock.last().path, "/v1/segments");

    mailtea
        .segments
        .list(json!({ "publication_id": "pub_1" }))
        .await
        .expect("list failed");
    assert_eq!(mock.last().method, "GET");

    mailtea
        .segments
        .get("seg_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("get failed");
    assert_eq!(mock.last().path, "/v1/segments/seg_1");

    // A nullable filter is cleared by sending it as null, not by omitting it.
    mailtea
        .segments
        .update(
            "seg_1",
            json!({ "publication_id": "pub_1", "status_filter": null }),
        )
        .await
        .expect("update failed");
    let request = mock.last();
    assert_eq!(request.method, "PATCH");
    assert_eq!(request.param("publication_id").as_deref(), Some("pub_1"));
    assert!(request.body["status_filter"].is_null());

    mailtea
        .segments
        .delete("seg_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("delete failed");
    assert_eq!(mock.last().method, "DELETE");
}

#[tokio::test]
async fn topics() {
    let (mock, mailtea) = common::client().await;

    let created = mailtea
        .topics
        .create(&CreateTopic::new("pub_1", "Weekly", "opt_in").visibility("public"))
        .await
        .expect("create failed");
    let request = mock.last();
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("POST", "/v1/topics")
    );
    assert_eq!(request.body["default_subscription"], "opt_in");
    assert_eq!(request.body["visibility"], "public");
    assert_eq!(created["name"], "Weekly");

    mailtea
        .topics
        .list(json!({ "publication_id": "pub_1" }))
        .await
        .expect("list failed");
    assert_eq!(mock.last().path, "/v1/topics");

    mailtea
        .topics
        .get("top_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("get failed");
    assert_eq!(mock.last().path, "/v1/topics/top_1");

    mailtea
        .topics
        .update(
            "top_1",
            json!({ "publication_id": "pub_1", "name": "Monthly" }),
        )
        .await
        .expect("update failed");
    assert_eq!(
        mock.last().param("publication_id").as_deref(),
        Some("pub_1")
    );

    mailtea
        .topics
        .delete("top_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("delete failed");
    assert_eq!(mock.last().method, "DELETE");
}

#[tokio::test]
async fn posts() {
    let (mock, mailtea) = common::client().await;

    let created = mailtea
        .posts
        .create(
            &CreatePost::new("pub_1", "Issue 1")
                .html("<p>Hi</p>")
                .kind("newsletter"),
        )
        .await
        .expect("create failed");
    let request = mock.last();
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("POST", "/v1/posts")
    );
    assert_eq!(request.body["subject"], "Issue 1");
    assert_eq!(request.body["kind"], "newsletter");
    assert_eq!(created["id"], "post_1");

    mailtea
        .posts
        .list(json!({ "publication_id": "pub_1", "limit": 10 }))
        .await
        .expect("list failed");
    assert_eq!(mock.last().param("limit").as_deref(), Some("10"));

    mailtea.posts.get("post_1", ()).await.expect("get failed");
    let request = mock.last();
    assert_eq!(request.path, "/v1/posts/post_1");
    assert_eq!(request.query, None, "`()` means no query at all");

    mailtea
        .posts
        .update("post_1", json!({ "subject": "Issue 1 (revised)" }))
        .await
        .expect("update failed");
    assert_eq!(mock.last().body["subject"], "Issue 1 (revised)");

    // Sending now sends NO body: the endpoint's schema takes an optional
    // `scheduled_at` and nothing else.
    let sent = mailtea.posts.send("post_1", ()).await.expect("send failed");
    let request = mock.last();
    assert_eq!(request.path, "/v1/posts/post_1/send");
    assert!(request.body.is_null());
    assert_eq!(sent["status"], "sending");

    mailtea
        .posts
        .send("post_1", json!({ "scheduled_at": "2026-09-01T09:00:00Z" }))
        .await
        .expect("scheduled send failed");
    assert_eq!(mock.last().body["scheduled_at"], "2026-09-01T09:00:00Z");

    let test = mailtea
        .posts
        .send_test(
            "post_1",
            &SendTestPost::new(["you@example.com"], "Acme <hello@acme.com>"),
        )
        .await
        .expect("send_test failed");
    let request = mock.last();
    assert_eq!(request.path, "/v1/posts/post_1/test");
    assert_eq!(request.body["recipients"], json!(["you@example.com"]));
    assert_eq!(test["sent_to"], json!(["you@example.com"]));

    mailtea
        .posts
        .delete("post_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("delete failed");
    assert_eq!(mock.last().method, "DELETE");
}

#[tokio::test]
async fn senders() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .senders
        .create(json!({ "publication_id": "pub_1", "name": "Acme", "email": "hi@acme.com" }))
        .await
        .expect("create failed");
    assert_eq!(mock.last().path, "/v1/senders");

    mailtea
        .senders
        .list(json!({ "publication_id": "pub_1" }))
        .await
        .expect("list failed");
    assert_eq!(mock.last().method, "GET");

    mailtea
        .senders
        .get("snd_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("get failed");
    assert_eq!(mock.last().path, "/v1/senders/snd_1");

    // Unlike the other PATCHes, this one carries `publication_id` in the body
    // only — the endpoint reads it from there.
    mailtea
        .senders
        .update(
            "snd_1",
            json!({ "publication_id": "pub_1", "name": "Acme Support" }),
        )
        .await
        .expect("update failed");
    let request = mock.last();
    assert_eq!(request.method, "PATCH");
    assert_eq!(request.query, None);
    assert_eq!(request.body["publication_id"], "pub_1");

    mailtea
        .senders
        .delete("snd_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("delete failed");
    assert_eq!(mock.last().method, "DELETE");
}

#[tokio::test]
async fn assets() {
    let (mock, mailtea) = common::client().await;

    let uploaded = mailtea
        .assets
        .upload(&UploadAsset::from_bytes(
            "pub_1",
            "hero.png",
            "image/png",
            b"hi",
        ))
        .await
        .expect("upload failed");
    let request = mock.last();
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("POST", "/v1/assets")
    );
    // Raw bytes are base64-encoded for you: "hi" -> "aGk=".
    assert_eq!(request.body["content"], "aGk=");
    assert_eq!(request.body["content_type"], "image/png");
    assert_eq!(uploaded["url"], "https://assets.example.invalid/ast_1.png");

    mailtea
        .assets
        .list(json!({ "publication_id": "pub_1", "search": "hero" }))
        .await
        .expect("list failed");
    assert_eq!(mock.last().param("search").as_deref(), Some("hero"));

    mailtea
        .assets
        .delete("ast_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("delete failed");
    let request = mock.last();
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("DELETE", "/v1/assets/ast_1")
    );
}

#[tokio::test]
async fn suppressions() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .suppressions
        .list(json!({ "reason": "bounce", "limit": 10 }))
        .await
        .expect("list failed");
    let request = mock.last();
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("GET", "/v1/suppressions")
    );
    assert_eq!(request.param("reason").as_deref(), Some("bounce"));

    let added = mailtea
        .suppressions
        .add(json!({ "emails": ["blocked@example.com"], "reason": "manual" }))
        .await
        .expect("add failed");
    assert_eq!(mock.last().method, "POST");
    assert_eq!(added["added"], 1);

    let removed = mailtea
        .suppressions
        .remove(json!({ "emails": ["blocked@example.com"] }))
        .await
        .expect("remove failed");
    let request = mock.last();
    assert_eq!(request.method, "DELETE");
    // A DELETE with a body — the addresses to remove ride in it.
    assert_eq!(request.body["emails"], json!(["blocked@example.com"]));
    assert_eq!(removed["removed"], 1);

    // The one endpoint that answers CSV, not JSON.
    let csv = mailtea.suppressions.export().await.expect("export failed");
    assert_eq!(mock.last().path, "/v1/suppressions/export");
    assert!(csv.starts_with("email,reason,source,created_at"));
}

#[tokio::test]
async fn templates() {
    let (mock, mailtea) = common::client().await;

    let rendered = mailtea
        .templates
        .render(json!({ "spec": { "blocks": [] }, "variables": { "name": "Ada" } }))
        .await
        .expect("render failed");
    assert_eq!(mock.last().path, "/v1/templates/render");
    assert_eq!(rendered["html"], "<p>Rendered</p>");

    mailtea
        .templates
        .create(json!({ "publication_id": "pub_1", "name": "Receipt", "html": "<p>Hi</p>" }))
        .await
        .expect("create failed");
    assert_eq!(mock.last().path, "/v1/templates");

    mailtea
        .templates
        .list(json!({ "publication_id": "pub_1" }))
        .await
        .expect("list failed");
    assert_eq!(mock.last().method, "GET");

    mailtea
        .templates
        .get("tpl_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("get failed");
    assert_eq!(mock.last().path, "/v1/templates/tpl_1");

    mailtea
        .templates
        .update(
            "tpl_1",
            json!({ "publication_id": "pub_1", "subject": null }),
        )
        .await
        .expect("update failed");
    let request = mock.last();
    assert_eq!(request.param("publication_id").as_deref(), Some("pub_1"));
    assert!(request.body["subject"].is_null(), "null clears the field");

    let published = mailtea
        .templates
        .publish("tpl_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("publish failed");
    let request = mock.last();
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("POST", "/v1/templates/tpl_1/publish")
    );
    assert_eq!(request.param("publication_id").as_deref(), Some("pub_1"));
    assert_eq!(published["status"], "published");

    mailtea
        .templates
        .unpublish("tpl_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("unpublish failed");
    assert_eq!(mock.last().path, "/v1/templates/tpl_1/unpublish");

    mailtea
        .templates
        .versions("tpl_1", json!({ "publication_id": "pub_1", "limit": 5 }))
        .await
        .expect("versions failed");
    assert_eq!(mock.last().path, "/v1/templates/tpl_1/versions");

    let restored = mailtea
        .templates
        .restore_version("tpl_1", 2, json!({ "publication_id": "pub_1" }))
        .await
        .expect("restore failed");
    assert_eq!(mock.last().path, "/v1/templates/tpl_1/versions/2/restore");
    // Restoring is a content write, so the template drops back to draft.
    assert_eq!(restored["unpublished"], true);

    mailtea
        .templates
        .duplicate("tpl_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("duplicate failed");
    assert_eq!(mock.last().path, "/v1/templates/tpl_1/duplicate");

    mailtea
        .templates
        .delete("tpl_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("delete failed");
    assert_eq!(mock.last().method, "DELETE");
}

#[tokio::test]
async fn domains_and_tracking_domains() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .domains
        .create(json!({ "publication_id": "pub_1", "domain": "acme.com" }))
        .await
        .expect("create failed");
    assert_eq!(mock.last().path, "/v1/domains");

    mailtea
        .domains
        .list(json!({ "publication_id": "pub_1" }))
        .await
        .expect("list failed");
    assert_eq!(mock.last().method, "GET");

    mailtea
        .domains
        .get("dom_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("get failed");
    assert_eq!(mock.last().path, "/v1/domains/dom_1");

    let verified = mailtea
        .domains
        .verify("dom_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("verify failed");
    assert_eq!(mock.last().path, "/v1/domains/dom_1/verify");
    assert_eq!(verified["status"], "verified");

    mailtea
        .domains
        .update(
            "dom_1",
            json!({ "publication_id": "pub_1", "custom_return_path": "bounces" }),
        )
        .await
        .expect("update failed");
    let request = mock.last();
    assert_eq!(request.param("publication_id").as_deref(), Some("pub_1"));
    assert_eq!(request.body["custom_return_path"], "bounces");

    // The tracking-domain create splits its payload: publication_id to the
    // query, and only `subdomain` in the body.
    mailtea
        .domains
        .tracking
        .create(
            "dom_1",
            json!({ "publication_id": "pub_1", "subdomain": "link" }),
        )
        .await
        .expect("tracking create failed");
    let request = mock.last();
    assert_eq!(request.path, "/v1/domains/dom_1/tracking-domains");
    assert_eq!(request.param("publication_id").as_deref(), Some("pub_1"));
    assert_eq!(request.body, json!({ "subdomain": "link" }));

    mailtea
        .domains
        .tracking
        .list("dom_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("tracking list failed");
    assert_eq!(mock.last().method, "GET");

    mailtea
        .domains
        .tracking
        .verify("dom_1", "trk_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("tracking verify failed");
    assert_eq!(
        mock.last().path,
        "/v1/domains/dom_1/tracking-domains/trk_1/verify"
    );

    mailtea
        .domains
        .tracking
        .delete("dom_1", "trk_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("tracking delete failed");
    let request = mock.last();
    assert_eq!(request.method, "DELETE");
    assert_eq!(request.path, "/v1/domains/dom_1/tracking-domains/trk_1");

    mailtea
        .domains
        .delete("dom_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("delete failed");
    assert_eq!(mock.last().path, "/v1/domains/dom_1");
}

#[tokio::test]
async fn webhooks() {
    let (mock, mailtea) = common::client().await;

    let created = mailtea
        .webhooks
        .create(json!({
            "publication_id": "pub_1",
            "url": "https://example.invalid/hook",
            "events": ["email.delivered"]
        }))
        .await
        .expect("create failed");
    let request = mock.last();
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("POST", "/v1/webhooks/endpoints")
    );
    // The signing secret comes back once, on create.
    assert!(created["signing_secret"]
        .as_str()
        .is_some_and(|secret| secret.starts_with("whsec_")));

    mailtea
        .webhooks
        .list(json!({ "publication_id": "pub_1" }))
        .await
        .expect("list failed");
    assert_eq!(mock.last().method, "GET");

    mailtea
        .webhooks
        .get("whe_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("get failed");
    assert_eq!(mock.last().path, "/v1/webhooks/endpoints/whe_1");

    mailtea
        .webhooks
        .update(
            "whe_1",
            json!({ "publication_id": "pub_1", "enabled": false }),
        )
        .await
        .expect("update failed");
    let request = mock.last();
    assert_eq!(request.param("publication_id").as_deref(), Some("pub_1"));
    assert_eq!(request.body["enabled"], false);

    mailtea
        .webhooks
        .delete("whe_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("delete failed");
    assert_eq!(mock.last().method, "DELETE");
}

#[tokio::test]
async fn contact_properties_and_api_keys() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .contact_properties
        .create(json!({ "key": "plan", "type": "string" }))
        .await
        .expect("create failed");
    let request = mock.last();
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("POST", "/v1/contact-properties")
    );
    // Team-scoped: there is no publication_id anywhere.
    assert_eq!(request.body.get("publication_id"), None);

    mailtea
        .contact_properties
        .list(())
        .await
        .expect("list failed");
    assert_eq!(mock.last().query, None);

    mailtea
        .contact_properties
        .update("cpr_1", json!({ "key": "tier" }))
        .await
        .expect("update failed");
    assert_eq!(mock.last().path, "/v1/contact-properties/cpr_1");

    mailtea
        .contact_properties
        .delete("cpr_1")
        .await
        .expect("delete failed");
    assert_eq!(mock.last().method, "DELETE");

    let key = mailtea
        .api_keys
        .create(json!({ "name": "CI", "permission": "sending_access" }))
        .await
        .expect("api key create failed");
    assert_eq!(mock.last().path, "/v1/api-keys");
    // The token is returned once.
    assert_eq!(key["token"], "mt_svc_returned_once");

    mailtea.api_keys.list().await.expect("api key list failed");
    assert_eq!(mock.last().method, "GET");

    mailtea
        .api_keys
        .revoke("key_1")
        .await
        .expect("revoke failed");
    let request = mock.last();
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("DELETE", "/v1/api-keys/key_1")
    );
}

#[tokio::test]
async fn automations() {
    let (mock, mailtea) = common::client().await;

    let validation = mailtea
        .automations
        .validate(json!({ "publication_id": "pub_1", "steps": [] }))
        .await
        .expect("validate failed");
    assert_eq!(mock.last().path, "/v1/automations/validate");
    assert_eq!(validation["valid"], true);

    mailtea
        .automations
        .create(json!({ "publication_id": "pub_1", "name": "Welcome", "steps": [] }))
        .await
        .expect("create failed");
    assert_eq!(mock.last().path, "/v1/automations");

    mailtea
        .automations
        .list(json!({ "publication_id": "pub_1", "status": "active" }))
        .await
        .expect("list failed");
    assert_eq!(mock.last().param("status").as_deref(), Some("active"));

    mailtea
        .automations
        .get("aut_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("get failed");
    assert_eq!(mock.last().path, "/v1/automations/aut_1");

    // `publication_id` is a query parameter here and must NOT ride in the body:
    // the endpoint's schema does not accept it there.
    mailtea
        .automations
        .update(
            "aut_1",
            json!({ "publication_id": "pub_1", "name": "Welcome v2" }),
        )
        .await
        .expect("update failed");
    let request = mock.last();
    assert_eq!(request.param("publication_id").as_deref(), Some("pub_1"));
    assert_eq!(request.body.get("publication_id"), None);
    assert_eq!(request.body["name"], "Welcome v2");

    let activated = mailtea
        .automations
        .activate("aut_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("activate failed");
    assert_eq!(mock.last().path, "/v1/automations/aut_1/activate");
    assert_eq!(activated["status"], "active");

    // Omitting `cancel_runs` sends NO body, so the per-verb default applies —
    // false on pause, true on archive.
    mailtea
        .automations
        .pause("aut_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("pause failed");
    let request = mock.last();
    assert_eq!(request.path, "/v1/automations/aut_1/pause");
    assert!(request.body.is_null());

    mailtea
        .automations
        .pause(
            "aut_1",
            json!({ "publication_id": "pub_1", "cancel_runs": true }),
        )
        .await
        .expect("pause failed");
    let request = mock.last();
    assert_eq!(request.body, json!({ "cancel_runs": true }));
    assert_eq!(request.param("publication_id").as_deref(), Some("pub_1"));

    mailtea
        .automations
        .archive("aut_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("archive failed");
    assert_eq!(mock.last().path, "/v1/automations/aut_1/archive");

    mailtea
        .automations
        .versions("aut_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("versions failed");
    assert_eq!(mock.last().path, "/v1/automations/aut_1/versions");

    let version = mailtea
        .automations
        .version("aut_1", 3, json!({ "publication_id": "pub_1" }))
        .await
        .expect("version failed");
    assert_eq!(mock.last().path, "/v1/automations/aut_1/versions/3");
    assert_eq!(version["version"], 3);

    let metrics = mailtea
        .automations
        .metrics("aut_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("metrics failed");
    assert_eq!(mock.last().path, "/v1/automations/aut_1/metrics");
    assert_eq!(metrics["excludes_test_runs"], true);

    let test = mailtea
        .automations
        .test(
            "aut_1",
            json!({ "publication_id": "pub_1", "email": "you@example.com" }),
        )
        .await
        .expect("test failed");
    let request = mock.last();
    assert_eq!(request.path, "/v1/automations/aut_1/test");
    assert_eq!(request.param("publication_id").as_deref(), Some("pub_1"));
    assert_eq!(request.body.get("publication_id"), None);
    assert_eq!(request.body["email"], "you@example.com");
    assert_eq!(test["is_test"], true);

    mailtea
        .automations
        .delete("aut_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("delete failed");
    assert_eq!(mock.last().method, "DELETE");
}

#[tokio::test]
async fn automation_runs() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .automation_runs
        .list(
            "aut_1",
            json!({
                "publication_id": "pub_1",
                "status": ["running", "waiting"],
                "is_test": false
            }),
        )
        .await
        .expect("list failed");
    let request = mock.last();
    assert_eq!(request.path, "/v1/automations/aut_1/runs");
    // A list of statuses is comma-joined for you.
    assert_eq!(request.param("status").as_deref(), Some("running,waiting"));
    // The server matches `is_test` against the literal strings "true"/"false".
    assert_eq!(request.param("is_test").as_deref(), Some("false"));

    mailtea
        .automation_runs
        .get("aut_1", "run_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("get failed");
    assert_eq!(mock.last().path, "/v1/automations/aut_1/runs/run_1");

    let canceled = mailtea
        .automation_runs
        .cancel("aut_1", "run_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("cancel failed");
    let request = mock.last();
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("POST", "/v1/automations/aut_1/runs/run_1/cancel")
    );
    assert_eq!(canceled["status"], "canceled");
}

#[tokio::test]
async fn events_and_event_definitions() {
    let (mock, mailtea) = common::client().await;

    let recorded = mailtea
        .events
        .send(json!({
            "publication_id": "pub_1",
            "name": "order.placed",
            "email": "reader@example.com",
            "properties": { "total": 42 }
        }))
        .await
        .expect("event send failed");
    let request = mock.last();
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("POST", "/v1/events")
    );
    assert_eq!(request.body["name"], "order.placed");
    assert_eq!(recorded["enrolled_automations"], 1);

    mailtea
        .events
        .list(json!({ "publication_id": "pub_1", "name": "order.placed" }))
        .await
        .expect("event list failed");
    assert_eq!(mock.last().param("name").as_deref(), Some("order.placed"));

    mailtea
        .event_definitions
        .create(json!({ "publication_id": "pub_1", "name": "order.placed" }))
        .await
        .expect("definition create failed");
    assert_eq!(mock.last().path, "/v1/event-definitions");

    mailtea
        .event_definitions
        .list(json!({ "publication_id": "pub_1" }))
        .await
        .expect("definition list failed");
    assert_eq!(mock.last().method, "GET");

    mailtea
        .event_definitions
        .get("evd_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("definition get failed");
    assert_eq!(mock.last().path, "/v1/event-definitions/evd_1");

    // `name` is immutable, so `publication_id` goes to the query and only the
    // mutable fields ride in the body.
    mailtea
        .event_definitions
        .update(
            "evd_1",
            json!({ "publication_id": "pub_1", "schema_json": null }),
        )
        .await
        .expect("definition update failed");
    let request = mock.last();
    assert_eq!(request.param("publication_id").as_deref(), Some("pub_1"));
    assert_eq!(request.body.get("publication_id"), None);
    assert!(request.body["schema_json"].is_null());

    mailtea
        .event_definitions
        .delete("evd_1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("definition delete failed");
    assert_eq!(mock.last().method, "DELETE");
}
