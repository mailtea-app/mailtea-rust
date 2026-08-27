//! Standard Webhooks ([standardwebhooks.com](https://www.standardwebhooks.com/))
//! signature verification.
//!
//! A port of the Mailtea signer — `packages/contracts/src/webhook-signing.ts`,
//! also mirrored in the Node SDK, the Python SDK and the webhook-ingester. Kept
//! in exact parity so a signature produced by the platform verifies here
//! byte-for-byte.
//!
//! The stored signing secret is `whsec_<base64>`; the HMAC key is the base64
//! remainder decoded to bytes. The signed content is
//! `{msg_id}.{timestamp}.{payload}` where `timestamp` is Unix SECONDS, matching
//! the `webhook-timestamp` header. The `webhook-signature` header is
//! `v1,<base64 HMAC-SHA256>`; during key rotation it may carry several
//! space-delimited `v1,<sig>` tokens and a match against any one of them passes.

use std::time::{SystemTime, UNIX_EPOCH};

use base64::alphabet;
use base64::engine::{general_purpose, DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use base64::Engine as _;
use hmac::{Hmac, Mac};
use sha2::Sha256;

const SECRET_PREFIX: &str = "whsec_";
const SIGNATURE_VERSION: &str = "v1";

/// Allowed clock skew each way, in seconds. Five minutes, the Standard Webhooks
/// default.
pub const DEFAULT_TOLERANCE_SECONDS: i64 = 300;

/// Matches Node's lenient base64 decoder: accepts the base64url alphabet and
/// tolerates missing padding, so a secret minted with either alphabet decodes to
/// the same bytes.
const LENIENT_BASE64: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new()
        .with_decode_allow_trailing_bits(true)
        .with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// How verification is tuned. [`Default`] is the 5-minute tolerance against the
/// real clock.
#[derive(Clone, Debug)]
pub struct VerifyOptions {
    /// Allowed clock skew each way. A delivery outside it is a replay.
    pub tolerance_seconds: i64,
    /// Injectable "now", in Unix seconds — for tests. `None` reads the clock.
    pub now: Option<i64>,
}

impl Default for VerifyOptions {
    fn default() -> Self {
        Self {
            tolerance_seconds: DEFAULT_TOLERANCE_SECONDS,
            now: None,
        }
    }
}

/// Anything a `webhook-timestamp` can arrive as. A header is a string; a test
/// usually has an integer.
pub trait IntoUnixSeconds {
    /// `None` for a value that is not a finite number of seconds — which
    /// [`verify_webhook_signature`] turns into a failed verification rather than
    /// a panic.
    fn into_unix_seconds(self) -> Option<i64>;
}

impl IntoUnixSeconds for i64 {
    fn into_unix_seconds(self) -> Option<i64> {
        Some(self)
    }
}

impl IntoUnixSeconds for u64 {
    fn into_unix_seconds(self) -> Option<i64> {
        i64::try_from(self).ok()
    }
}

impl IntoUnixSeconds for f64 {
    fn into_unix_seconds(self) -> Option<i64> {
        self.is_finite().then(|| self.floor() as i64)
    }
}

impl IntoUnixSeconds for &str {
    fn into_unix_seconds(self) -> Option<i64> {
        let trimmed = self.trim();
        trimmed
            .parse::<i64>()
            .ok()
            // A fractional timestamp is floored, the way the Python SDK does it.
            .or_else(|| trimmed.parse::<f64>().ok().and_then(f64::into_unix_seconds))
    }
}

impl IntoUnixSeconds for &String {
    fn into_unix_seconds(self) -> Option<i64> {
        self.as_str().into_unix_seconds()
    }
}

/// Sign a webhook payload.
///
/// Returns the `webhook-signature` header value in Standard Webhooks form,
/// `v1,<base64 HMAC-SHA256>`. Useful for faking Mailtea deliveries in tests.
///
/// `timestamp` is Unix seconds — the same value sent in `webhook-timestamp`.
pub fn sign_webhook(
    secret: &str,
    msg_id: &str,
    timestamp: impl IntoUnixSeconds,
    payload: &str,
) -> String {
    let seconds = timestamp.into_unix_seconds().unwrap_or(0);
    format!(
        "{SIGNATURE_VERSION},{}",
        compute_signature(secret, msg_id, seconds, payload)
    )
}

/// Verify a `webhook-signature` header against the expected HMAC, with the
/// default 5-minute replay window.
///
/// Pass the **raw** request body, exactly as received — not re-serialized JSON.
///
/// ```
/// use mailtea::{sign_webhook, verify_webhook_signature};
///
/// let secret = "whsec_dGVzdHNpZ25pbmdrZXlub3RhcmVhbHNlY3JldA==";
/// let body = r#"{"type":"email.delivered"}"#;
/// let timestamp = 1_756_000_000_i64;
/// let header = sign_webhook(secret, "msg_1", timestamp, body);
///
/// // Real code reads `now` from the clock; this pins it so the doctest cannot
/// // age out.
/// use mailtea::{verify_webhook_signature_with, VerifyOptions};
/// assert!(verify_webhook_signature_with(
///     secret,
///     "msg_1",
///     timestamp,
///     body,
///     &header,
///     &VerifyOptions { now: Some(timestamp), ..Default::default() },
/// ));
///
/// // A tampered body never verifies.
/// assert!(!verify_webhook_signature_with(
///     secret,
///     "msg_1",
///     timestamp,
///     r#"{"type":"email.bounced"}"#,
///     &header,
///     &VerifyOptions { now: Some(timestamp), ..Default::default() },
/// ));
///
/// # let _ = verify_webhook_signature(secret, "msg_1", timestamp, body, &header);
/// ```
pub fn verify_webhook_signature(
    secret: &str,
    msg_id: &str,
    timestamp: impl IntoUnixSeconds,
    payload: &str,
    signature_header: &str,
) -> bool {
    verify_webhook_signature_with(
        secret,
        msg_id,
        timestamp,
        payload,
        signature_header,
        &VerifyOptions::default(),
    )
}

/// [`verify_webhook_signature`] with the tolerance and clock under your control.
///
/// The header may carry multiple space-delimited `v1,<sig>` tokens (Standard
/// Webhooks allows key rotation — the platform may sign a delivery with both the
/// old and the new secret); a match against any `v1` token passes. Returns
/// `false` when the timestamp is outside the tolerance. The comparison is
/// constant-time, and a bad signature is a `false`, never a panic.
pub fn verify_webhook_signature_with(
    secret: &str,
    msg_id: &str,
    timestamp: impl IntoUnixSeconds,
    payload: &str,
    signature_header: &str,
    options: &VerifyOptions,
) -> bool {
    let Some(seconds) = timestamp.into_unix_seconds() else {
        return false;
    };

    let now = options.now.unwrap_or_else(unix_now);
    // Saturating, not plain, arithmetic: `webhook-timestamp` is attacker-
    // controlled, and `i64::MIN` would overflow both the subtraction and the
    // `abs()` — a panic in any build with overflow checks on, where this has to
    // be a `false` like every other bad header.
    if now.saturating_sub(seconds).saturating_abs() > options.tolerance_seconds {
        return false;
    }

    let signed_content = signed_content(msg_id, seconds, payload);
    let key = decode_signing_key(secret);

    signature_header.split(' ').any(|token| {
        let Some((version, signature)) = token.split_once(',') else {
            return false;
        };
        if version != SIGNATURE_VERSION {
            return false;
        }
        let Ok(expected) = LENIENT_BASE64.decode(signature.trim()) else {
            return false;
        };
        // `verify_slice` is the constant-time compare: it diffs the whole
        // digest, so a signature that matches the first byte takes exactly as
        // long to reject as one that matches none.
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(&key) else {
            return false;
        };
        mac.update(signed_content.as_bytes());
        mac.verify_slice(&expected).is_ok()
    })
}

fn signed_content(msg_id: &str, timestamp: i64, payload: &str) -> String {
    format!("{msg_id}.{timestamp}.{payload}")
}

fn compute_signature(secret: &str, msg_id: &str, timestamp: i64, payload: &str) -> String {
    let key = decode_signing_key(secret);
    let mut mac = Hmac::<Sha256>::new_from_slice(&key).expect("HMAC accepts a key of any length");
    mac.update(signed_content(msg_id, timestamp, payload).as_bytes());
    general_purpose::STANDARD.encode(mac.finalize().into_bytes())
}

/// Decode the HMAC key from a `whsec_`-prefixed secret.
///
/// A secret that is not valid base64 at all is used as raw bytes rather than
/// rejected — the same thing the other ports do, so a misconfigured secret
/// fails verification (which is safe) instead of panicking on startup.
fn decode_signing_key(secret: &str) -> Vec<u8> {
    let raw = secret.strip_prefix(SECRET_PREFIX).unwrap_or(secret).trim();
    let normalized: String = raw
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '=')
        .map(|c| match c {
            '-' => '+',
            '_' => '/',
            other => other,
        })
        .collect();
    LENIENT_BASE64
        .decode(&normalized)
        .unwrap_or_else(|_| raw.as_bytes().to_vec())
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}
