//! Endpoint parity with the other official Mailtea SDKs.
//!
//! The canonical endpoint set is what the Python SDK calls. This test pins that
//! list here — rather than reading it from the monorepo — so the mirrored
//! `mailtea-app/mailtea-rust` repo stays standalone and this test still means
//! something after a `git clone`.
//!
//! The list was extracted with:
//!
//! ```text
//! grep -ohE '"/v1/[^"]*"' sdks/python/mailtea/*.py | sort -u
//! ```
//!
//! run in `mailtea-app/mailtea` at the 0.9.1 release of the Python SDK. A
//! trailing slash means "…/:id and everything under it" — that is the
//! granularity the grep produces, and matching it keeps the two lists
//! comparable.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// The 37 path prefixes the Python SDK reaches. See the module comment.
const PYTHON_ENDPOINTS: &[&str] = &[
    "/v1/api-keys",
    "/v1/api-keys/",
    "/v1/assets",
    "/v1/assets/",
    "/v1/automations",
    "/v1/automations/",
    "/v1/automations/validate",
    "/v1/contact-properties",
    "/v1/contact-properties/",
    "/v1/contacts",
    "/v1/contacts/",
    "/v1/domains",
    "/v1/domains/",
    "/v1/domains/claim",
    "/v1/domains/claims/",
    "/v1/emails",
    "/v1/emails/",
    "/v1/emails/analytics",
    "/v1/emails/batch",
    "/v1/emails/inbound",
    "/v1/event-definitions",
    "/v1/event-definitions/",
    "/v1/events",
    "/v1/posts",
    "/v1/posts/",
    "/v1/segments",
    "/v1/segments/",
    "/v1/senders",
    "/v1/senders/",
    "/v1/suppressions",
    "/v1/suppressions/export",
    "/v1/templates",
    "/v1/templates/",
    "/v1/templates/render",
    "/v1/topics",
    "/v1/topics/",
    "/v1/webhooks/endpoints",
];

/// Endpoints this SDK reaches that the Python SDK does not. Empty on purpose:
/// an SDK that invents API surface is worse than one that lags it. Add a path
/// here — with a comment saying why — only when the Rust SDK deliberately gets
/// there first.
const EXPECTED_EXTRA: &[&str] = &[];

#[test]
fn every_python_endpoint_is_reachable_from_this_sdk() {
    let ours = endpoints_in_source();

    let missing: Vec<&&str> = PYTHON_ENDPOINTS
        .iter()
        .filter(|endpoint| !ours.contains(**endpoint))
        .collect();
    assert!(
        missing.is_empty(),
        "these endpoints are in the Python SDK but not reachable here: {missing:?}"
    );
    assert_eq!(
        PYTHON_ENDPOINTS.len(),
        37,
        "the pinned list should still be the 37 the grep produced"
    );
}

#[test]
fn this_sdk_invents_no_endpoints() {
    let ours = endpoints_in_source();
    let known: BTreeSet<&str> = PYTHON_ENDPOINTS
        .iter()
        .chain(EXPECTED_EXTRA.iter())
        .copied()
        .collect();

    let extra: Vec<&String> = ours
        .iter()
        .filter(|endpoint| !known.contains(endpoint.as_str()))
        .collect();
    assert!(
        extra.is_empty(),
        "these endpoints exist here but in no other SDK — if that is deliberate, \
         add them to EXPECTED_EXTRA with a reason: {extra:?}"
    );
}

/// Every `/v1/...` path prefix this crate's source builds a request from.
///
/// Reads the crate's own `src/` (via `CARGO_MANIFEST_DIR`, so it works from any
/// working directory) and pulls out the string literals that start a path. A
/// literal is cut at its first `{` or `?`, because `format!("/v1/emails/{}/cancel")`
/// is the `/v1/emails/` prefix in the grep's terms, and a query string is not
/// part of the endpoint.
fn endpoints_in_source() -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut files = Vec::new();
    collect_rust_files(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(
        files.len() > 15,
        "expected to scan every resource module, found {} files",
        files.len()
    );

    for file in files {
        let source = fs::read_to_string(&file).expect("a source file this crate compiled");
        for (index, _) in source.match_indices("\"/v1/") {
            let after_quote = &source[index + 1..];
            let Some(end) = after_quote.find('"') else {
                continue;
            };
            let literal = &after_quote[..end];
            let cut = literal.find(['{', '?']).unwrap_or(literal.len());
            found.insert(literal[..cut].to_string());
        }
    }
    found
}

fn collect_rust_files(dir: impl AsRef<Path>, out: &mut Vec<std::path::PathBuf>) {
    let entries = fs::read_dir(dir).expect("the crate's src/ directory");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}
