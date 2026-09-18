//! Messages built from response bodies captured from the live API.
//!
//! Every body here was recorded from `api.typesafe.ai` rather than invented, so
//! these assertions describe what the service actually returns. A hand-written
//! fixture would only prove the code agrees with itself.

use typesafe_sdk_error::{ApiErrorKind, Body, Error};
use typesafe_sdk_headers::Headers;

fn error_for(status: u16, raw: &str) -> Error {
    Error::from_response(status, Body::parse(raw), &Headers::new())
}

#[test]
fn an_authentication_failure_reads_the_nested_message() {
    let error = error_for(
        401,
        r#"{"detail":{"error_type":"authentication_error","message":"Cannot authenticate with the server. Please check your API key and try again."}}"#,
    );
    assert_eq!(
        error.to_string(),
        "401 Cannot authenticate with the server. Please check your API key and try again."
    );
    assert_eq!(
        error.api().map(|e| e.kind),
        Some(ApiErrorKind::Authentication)
    );
}

/// Sending no key at all is a 403, not a 401 — the API distinguishes
/// "credentials rejected" from "credentials absent".
#[test]
fn a_missing_key_is_permission_denied() {
    let error = error_for(
        403,
        r#"{"detail":{"error_type":"authentication_error","message":"Must supply an API key! Check your request and try again."}}"#,
    );
    assert_eq!(
        error.to_string(),
        "403 Must supply an API key! Check your request and try again."
    );
    assert_eq!(
        error.api().map(|e| e.kind),
        Some(ApiErrorKind::PermissionDenied)
    );
}

#[test]
fn an_unknown_model_is_a_bad_request() {
    let error = error_for(
        400,
        r#"{"detail":{"error_type":"api_usage_error","message":"Unknown model: no-such-model"}}"#,
    );
    assert_eq!(error.to_string(), "400 Unknown model: no-such-model");
    assert_eq!(error.api().map(|e| e.kind), Some(ApiErrorKind::BadRequest));
}

#[test]
fn a_validation_list_is_addressed_by_location() {
    let error = error_for(
        422,
        r#"{"detail":[{"type":"list_type","loc":["body","questions","q","score","criteria"],"msg":"Input should be a valid list","input":{"0":"a"}}]}"#,
    );
    assert_eq!(
        error.to_string(),
        "422 questions.q.score.criteria: Input should be a valid list"
    );
}

#[test]
fn an_empty_questions_object_reports_its_own_location() {
    let error = error_for(
        422,
        r#"{"detail":[{"type":"too_short","loc":["body","questions"],"msg":"Dictionary should have at least 1 item after validation, not 0","input":{}}]}"#,
    );
    assert_eq!(
        error.to_string(),
        "422 questions: Dictionary should have at least 1 item after validation, not 0"
    );
}

#[test]
fn several_validation_entries_are_joined() {
    let error = error_for(
        422,
        r#"{"detail":[{"loc":["body","questions","q"],"msg":"first"},{"loc":[],"msg":"second"}]}"#,
    );
    assert_eq!(error.to_string(), "422 questions.q: first; second");
}

#[test]
fn an_empty_body_says_so() {
    let error = error_for(429, "");
    assert_eq!(error.to_string(), "429 status code (no body)");
    assert_eq!(error.api().map(|e| e.kind), Some(ApiErrorKind::RateLimit));
}

#[test]
fn an_unparseable_body_is_quoted_back() {
    let error = error_for(502, "<h1>bad gateway</h1>");
    assert_eq!(error.to_string(), "502 <h1>bad gateway</h1>");
    assert_eq!(
        error.api().map(|e| e.kind),
        Some(ApiErrorKind::InternalServer)
    );
}

/// A JSON body with no recognised message field is quoted as JSON.
///
/// Only this path truncates. A plain-text body is returned whole, because the
/// text *is* the message — matching `extractMessage` in the JS SDK, which
/// short-circuits on a string before the raw-body fallback is reached.
#[test]
fn an_unrecognised_json_body_is_quoted_as_json() {
    assert_eq!(
        error_for(400, r#"{"code":7}"#).to_string(),
        r#"400 {"code":7}"#
    );
}

#[test]
fn a_long_json_body_is_cut_at_two_hundred_characters() {
    let raw = format!(r#"{{"blob":"{}"}}"#, "x".repeat(500));
    let message = error_for(400, &raw).to_string();
    assert_eq!(message.chars().count(), "400 ".len() + 200 + 1);
    assert!(message.ends_with('\u{2026}'));
}

/// A text body is not truncated, however long, because it is the message.
#[test]
fn a_long_text_body_is_kept_whole() {
    let raw = "x".repeat(250);
    assert_eq!(error_for(500, &raw).to_string().chars().count(), 4 + 250);
}

#[test]
fn an_unmapped_status_stays_unmapped() {
    assert_eq!(
        error_for(418, "").api().map(|e| e.kind),
        Some(ApiErrorKind::Other)
    );
}

#[test]
fn a_timeout_is_also_a_connection_failure() {
    let timeout = Error::Timeout { timeout_ms: 10_000 };
    assert!(timeout.is_connection());
    assert_eq!(timeout.to_string(), "Request timed out after 10000ms.");
    assert!(!Error::aborted().is_connection());
    assert!(Error::aborted().is_user_abort());
}

#[test]
fn a_request_id_is_carried_through() {
    let headers = Headers::new().with("x-typesafe-request-id", "req_01a0b556");
    let error = Error::from_response(500, Body::Empty, &headers);
    assert_eq!(
        error.api().and_then(|e| e.request_id.as_deref()),
        Some("req_01a0b556")
    );
}
