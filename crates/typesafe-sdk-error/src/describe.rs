//! Turning a response body into one readable sentence.
//!
//! The API answers with several shapes depending on which layer rejected the
//! request: a framework validation list, a domain error object, or an
//! unstructured page from a proxy. Each is tried in the order the JS SDK tries
//! them, so the same request produces the same message in both SDKs.

use serde_json::Value;

use crate::body::Body;

/// Raw bodies longer than this are cut, so a stack trace stays readable.
const MAX_RAW_BODY: usize = 200;

/// The full error message for a status and body.
#[must_use]
pub fn describe(status: u16, body: &Body) -> String {
    if let Some(detail) = extract_message(body) {
        return format!("{status} {detail}");
    }
    match body {
        Body::Empty => format!("{status} status code (no body)"),
        Body::Text(raw) => format!("{status} {}", truncate(raw)),
        Body::Json(value) => format!("{status} {}", truncate(&value.to_string())),
    }
}

/// Cuts at [`MAX_RAW_BODY`] characters, marking that it was cut.
fn truncate(raw: &str) -> String {
    if raw.chars().count() <= MAX_RAW_BODY {
        return raw.to_owned();
    }
    let kept: String = raw.chars().take(MAX_RAW_BODY).collect();
    format!("{kept}\u{2026}")
}

/// The most specific message the body offers, if any.
fn extract_message(body: &Body) -> Option<String> {
    match body {
        Body::Text(raw) => (!raw.is_empty()).then(|| raw.clone()),
        Body::Json(value) => from_json(value),
        Body::Empty => None,
    }
}

/// Walks the known error shapes in precedence order.
fn from_json(value: &Value) -> Option<String> {
    let object = value.as_object()?;
    let field = |name: &str| object.get(name);

    string_of(field("error"))
        .or_else(|| nested_message(field("error")))
        .or_else(|| string_of(field("message")))
        .or_else(|| string_of(field("detail")))
        .or_else(|| nested_message(field("detail")))
        .or_else(|| validation_errors(field("detail")))
}

/// A field that is itself a string.
fn string_of(value: Option<&Value>) -> Option<String> {
    value?.as_str().map(ToOwned::to_owned)
}

/// A field that is an object carrying a `message`.
fn nested_message(value: Option<&Value>) -> Option<String> {
    string_of(value?.as_object()?.get("message"))
}

/// A framework validation list, rendered as `path: message` entries.
fn validation_errors(value: Option<&Value>) -> Option<String> {
    let entries: Vec<String> = value?
        .as_array()?
        .iter()
        .filter_map(validation_entry)
        .collect();
    (!entries.is_empty()).then(|| entries.join("; "))
}

/// One validation entry, addressed by its location within the request body.
fn validation_entry(entry: &Value) -> Option<String> {
    let object = entry.as_object()?;
    let message = object.get("msg")?.as_str()?;
    let location = object
        .get("loc")
        .and_then(Value::as_array)
        .map(|parts| join_location(parts))
        .unwrap_or_default();
    Some(if location.is_empty() {
        message.to_owned()
    } else {
        format!("{location}: {message}")
    })
}

/// Joins a location path, dropping the uninformative leading `body`.
fn join_location(parts: &[Value]) -> String {
    parts
        .iter()
        .filter_map(|part| match part {
            Value::String(text) if text == "body" => None,
            Value::String(text) => Some(text.clone()),
            other => Some(other.to_string()),
        })
        .collect::<Vec<_>>()
        .join(".")
}
