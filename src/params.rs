//! Turning what a caller passed into a path, a query string, and a body.
//!
//! Every method takes `impl Serialize`, so a caller may hand over a typed
//! request struct, a `serde_json::json!({...})` literal, or `()` for nothing at
//! all. These helpers are what flattens those three into one shape.

use serde::Serialize;
use serde_json::{Map, Value};

use crate::error::{Error, Result};

/// Serialize whatever the caller passed into a `Value`. `()` becomes
/// `Value::Null`, which every helper below reads as "nothing was given".
pub(crate) fn to_value(params: impl Serialize) -> Result<Value> {
    serde_json::to_value(params).map_err(|error| {
        Error::client(
            format!("could not serialize the request parameters: {error}"),
            "invalid_params",
        )
        .with_source(error)
    })
}

/// The JSON body for a request, or `None` when the caller passed nothing.
pub(crate) fn to_body(params: impl Serialize) -> Result<Option<Vec<u8>>> {
    let value = to_value(params)?;
    body_from_value(&value)
}

pub(crate) fn body_from_value(value: &Value) -> Result<Option<Vec<u8>>> {
    if value.is_null() {
        return Ok(None);
    }
    serde_json::to_vec(value).map(Some).map_err(|error| {
        Error::client(
            format!("could not serialize the request body: {error}"),
            "invalid_params",
        )
        .with_source(error)
    })
}

/// An empty object means "no body" for the handful of endpoints whose schema
/// rejects `{}` — `POST /v1/posts/:id/send` with nothing to say, for instance.
pub(crate) fn body_or_none(value: &Value) -> Result<Option<Vec<u8>>> {
    match value {
        Value::Object(map) if map.is_empty() => Ok(None),
        other => body_from_value(other),
    }
}

/// Render a query string (`""` or `"?a=b&c=d"`) from a serialized object,
/// dropping nulls the way Python's `_query` does.
pub(crate) fn query(params: impl Serialize) -> Result<String> {
    Ok(query_of_value(&to_value(params)?))
}

pub(crate) fn query_of_value(value: &Value) -> String {
    let Some(map) = value.as_object() else {
        return String::new();
    };
    let mut out = String::new();
    for (key, value) in map {
        if value.is_null() {
            continue;
        }
        out.push(if out.is_empty() { '?' } else { '&' });
        out.push_str(&encode(key));
        out.push('=');
        out.push_str(&encode(&scalar(value)));
    }
    out
}

/// Just these keys, in this order — for the endpoints that want
/// `publication_id` in the query while the rest of the payload goes in the body.
pub(crate) fn query_pick(value: &Value, keys: &[&str]) -> String {
    let mut picked = Map::new();
    if let Some(map) = value.as_object() {
        for key in keys {
            if let Some(found) = map.get(*key) {
                picked.insert((*key).to_string(), found.clone());
            }
        }
    }
    query_of_value(&Value::Object(picked))
}

/// Remove a key from the payload and hand it back — for the endpoints where a
/// field belongs in the query and must NOT also ride in the body.
pub(crate) fn take(value: &mut Value, key: &str) -> Value {
    value
        .as_object_mut()
        .and_then(|map| map.remove(key))
        .unwrap_or(Value::Null)
}

/// One query value as text.
///
/// A string goes through untouched; numbers and booleans stringify. An array is
/// joined with commas, which is the form the API reads (`status=queued,sent`) —
/// and what `automation_runs.list` documents. Anything else is sent as JSON
/// rather than dropped, so a mistake is visible in the request instead of
/// silently missing from it.
fn scalar(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        Value::Array(items) => items.iter().map(scalar).collect::<Vec<_>>().join(","),
        other => other.to_string(),
    }
}

/// Percent-encode one path segment or query value.
///
/// A real `txemail_…` id passes through untouched. An id that came from
/// somewhere less trustworthy — a URL, a form, a row someone else writes — must
/// not be able to walk out of the segment it belongs in: interpolated raw, an
/// id of `../domains` calls a different endpoint than the method that took it.
/// Everything outside the RFC 3986 unreserved set is escaped, `/` included.
pub(crate) fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn nulls_are_dropped_from_the_query() {
        let rendered = query(json!({ "limit": 10, "cursor": null })).unwrap();
        assert_eq!(rendered, "?limit=10");
    }

    #[test]
    fn no_params_is_no_query() {
        assert_eq!(query(()).unwrap(), "");
        assert_eq!(query(json!({})).unwrap(), "");
    }

    #[test]
    fn arrays_are_comma_joined() {
        let rendered = query(json!({ "status": ["queued", "sent"] })).unwrap();
        assert_eq!(rendered, "?status=queued%2Csent");
    }

    #[test]
    fn a_path_segment_cannot_escape_itself() {
        assert_eq!(encode("../domains"), "..%2Fdomains");
        assert_eq!(encode("txemail_abc123"), "txemail_abc123");
    }
}
