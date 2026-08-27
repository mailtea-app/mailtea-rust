//! Constructing the client: where the key and base URL come from, what a
//! missing key does, and that neither is ever printed.

mod common;

use common::API_KEY;
use mailtea::{Mailtea, DEFAULT_BASE_URL};
use serde_json::json;

#[test]
fn a_missing_key_is_a_client_error_not_a_panic() {
    // Nothing is read from the environment here — the builder is given an empty
    // key, which is the same as no key.
    let error = Mailtea::builder()
        .api_key("")
        .base_url("http://127.0.0.1:9")
        .build()
        .expect_err("an empty key should not build");

    assert_eq!(error.status(), 0);
    assert_eq!(error.code(), Some("missing_api_key"));
    assert!(error.message().contains("MAILTEA_API_KEY"));
}

/// Every environment-dependent assertion lives in this one test, because
/// `set_var` is process-global and Rust runs tests in parallel threads.
#[test]
fn the_environment_supplies_the_key_and_the_base_url() {
    let restore_key = std::env::var("MAILTEA_API_KEY").ok();
    let restore_base = std::env::var("MAILTEA_API_BASE_URL").ok();

    std::env::remove_var("MAILTEA_API_KEY");
    std::env::remove_var("MAILTEA_API_BASE_URL");
    let error = Mailtea::from_env().expect_err("no key in the environment");
    assert_eq!(error.code(), Some("missing_api_key"));

    std::env::set_var("MAILTEA_API_KEY", API_KEY);
    let from_env = Mailtea::from_env().expect("the key is in the environment now");
    assert_eq!(from_env.base_url(), DEFAULT_BASE_URL);

    // A base URL in the environment is what points a client at a self-hosted or
    // local Mailtea, and a trailing slash must not produce `//v1/emails`.
    std::env::set_var("MAILTEA_API_BASE_URL", "http://127.0.0.1:7787/");
    let local = Mailtea::from_env().expect("build failed");
    assert_eq!(local.base_url(), "http://127.0.0.1:7787");

    // An explicit base URL wins over the environment.
    let explicit = Mailtea::builder()
        .api_key(API_KEY)
        .base_url("https://self-hosted.invalid")
        .build()
        .expect("build failed");
    assert_eq!(explicit.base_url(), "https://self-hosted.invalid");

    match restore_key {
        Some(value) => std::env::set_var("MAILTEA_API_KEY", value),
        None => std::env::remove_var("MAILTEA_API_KEY"),
    }
    match restore_base {
        Some(value) => std::env::set_var("MAILTEA_API_BASE_URL", value),
        None => std::env::remove_var("MAILTEA_API_BASE_URL"),
    }
}

#[test]
fn debug_never_prints_the_api_key() {
    let mailtea = Mailtea::builder()
        .api_key(API_KEY)
        .base_url("https://api.mailtea.app")
        .build()
        .expect("build failed");

    let printed = format!("{mailtea:?}");
    assert!(!printed.contains(API_KEY), "the key leaked: {printed}");
    assert!(printed.contains("<redacted>"));

    let builder = format!("{:?}", Mailtea::builder().api_key(API_KEY));
    assert!(!builder.contains(API_KEY), "the key leaked: {builder}");
}

#[tokio::test]
async fn request_reaches_an_endpoint_the_sdk_does_not_wrap() {
    let (mock, mailtea) = common::client().await;

    // The escape hatch: any path, any body, deserialized into anything.
    let listed: serde_json::Value = mailtea
        .request("GET", "/v1/senders?publication_id=pub_1", ())
        .await
        .expect("request failed");

    let request = mock.last();
    assert_eq!(request.path, "/v1/senders");
    assert_eq!(request.param("publication_id").as_deref(), Some("pub_1"));
    assert_eq!(listed["object"], "list");
}

#[tokio::test]
async fn a_client_is_cheap_to_clone_and_shares_one_transport() {
    let (mock, mailtea) = common::client().await;

    let cloned = mailtea.clone();
    cloned
        .contacts
        .list(json!({ "publication_id": "pub_1" }))
        .await
        .expect("list failed");

    // The clone recorded against the same mock, which is the same transport.
    assert_eq!(mock.requests().len(), 1);
}
