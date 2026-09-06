//! Domain claims — take a domain back from the team that currently holds it.

mod common;

use serde_json::json;

#[tokio::test]
async fn claim_create_posts_the_claim() {
    let (mock, mailtea) = common::client().await;

    let claim = mailtea
        .domains
        .claims
        .create(json!({
            "publication_id": "pub_1",
            "name": "acme.com",
            "region": "eu-west-1"
        }))
        .await
        .expect("claim create failed");

    let request = mock.last();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/domains/claim");
    assert_eq!(request.body["name"], "acme.com");
    assert_eq!(request.body["region"], "eu-west-1");
    // The TXT record to publish arrives in `records`, never as a bare `txt`.
    assert_eq!(claim["records"][0]["record"], "Claim");
    assert_eq!(claim["status"], "pending");
}

#[tokio::test]
async fn claim_get_verify_and_cancel_reach_their_routes() {
    let (mock, mailtea) = common::client().await;
    let scope = json!({ "publication_id": "pub_1" });

    mailtea
        .domains
        .claims
        .get("clm_1", &scope)
        .await
        .expect("get failed");
    let request = mock.last();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/v1/domains/claims/clm_1");
    assert_eq!(request.param("publication_id").as_deref(), Some("pub_1"));

    // Verify answers with the claim AND the domain it produced, so the claimant
    // can publish its DNS without a second request.
    let verified = mailtea
        .domains
        .claims
        .verify("clm_1", &scope)
        .await
        .expect("verify failed");
    let request = mock.last();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/domains/claims/clm_1/verify");
    assert_eq!(verified["domain"]["id"], "dom_2");
    assert_eq!(verified["domain_id"], "dom_2");

    mailtea
        .domains
        .claims
        .cancel("clm_1", &scope)
        .await
        .expect("cancel failed");
    let request = mock.last();
    assert_eq!(request.method, "DELETE");
    assert_eq!(request.path, "/v1/domains/claims/clm_1");
    assert_eq!(request.param("publication_id").as_deref(), Some("pub_1"));
}

#[tokio::test]
async fn the_claim_id_is_percent_encoded_into_its_path_segment() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .domains
        .claims
        .get("clm/1", json!({ "publication_id": "pub_1" }))
        .await
        .expect("get failed");
    assert_eq!(mock.last().path, "/v1/domains/claims/clm%2F1");
}

// Pass-through: the multi-region fields need no code, and this fails if that
// ever stops being true.
#[tokio::test]
async fn domain_create_forwards_region_tls_and_tracking_subdomain() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .domains
        .create(json!({
            "publication_id": "pub_1",
            "name": "acme.com",
            "region": "ap-southeast-1",
            "tls": "enforced",
            "tracking_subdomain": "links"
        }))
        .await
        .expect("create failed");

    let body = mock.last().body;
    assert_eq!(body["region"], "ap-southeast-1");
    assert_eq!(body["tls"], "enforced");
    assert_eq!(body["tracking_subdomain"], "links");
}

#[tokio::test]
async fn domain_update_forwards_tls_and_tracking_subdomain() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .domains
        .update(
            "dom_1",
            json!({
                "publication_id": "pub_1",
                "tls": "enforced",
                "tracking_subdomain": "links"
            }),
        )
        .await
        .expect("update failed");

    let request = mock.last();
    assert_eq!(request.path, "/v1/domains/dom_1");
    assert_eq!(request.body["tls"], "enforced");
    assert_eq!(request.body["tracking_subdomain"], "links");
}

// The removal has to reach the wire AS null. The query builder drops nulls; the
// body must not, or "remove it" becomes "leave it alone" and the caller gets a
// 200 saying nothing happened.
#[tokio::test]
async fn domain_update_sends_an_explicit_null_to_clear_the_tracking_subdomain() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .domains
        .update(
            "dom_1",
            json!({ "publication_id": "pub_1", "tracking_subdomain": null }),
        )
        .await
        .expect("update failed");

    let request = mock.last();
    assert_eq!(request.path, "/v1/domains/dom_1");
    assert_eq!(request.param("publication_id").as_deref(), Some("pub_1"));
    // `get`, not indexing: a missing key also reads as Null, and the whole
    // point is that the key is present and null.
    assert_eq!(
        request.body.get("tracking_subdomain"),
        Some(&serde_json::Value::Null)
    );
}

// Three states, not two: an absent key leaves the subdomain alone, null removes
// it. A body that always carried the key would clear it on every update.
#[tokio::test]
async fn domain_update_omits_the_tracking_subdomain_when_it_is_not_named() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .domains
        .update(
            "dom_1",
            json!({ "publication_id": "pub_1", "tls": "enforced" }),
        )
        .await
        .expect("update failed");

    assert_eq!(mock.last().body.get("tracking_subdomain"), None);
}

#[tokio::test]
async fn domain_list_forwards_the_region_and_status_filters() {
    let (mock, mailtea) = common::client().await;

    mailtea
        .domains
        .list(json!({
            "publication_id": "pub_1",
            "region": "eu-west-1",
            "status": "verified"
        }))
        .await
        .expect("list failed");

    let request = mock.last();
    assert_eq!(request.param("region").as_deref(), Some("eu-west-1"));
    assert_eq!(request.param("status").as_deref(), Some("verified"));
}
