//! Per-call overrides, and the metadata a response carries.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot fail loudly is not a test"
)]

use std::sync::Arc;

use typesafe_sdk_client::{Client, RequestOptions};
use typesafe_sdk_config::Builder;
use typesafe_sdk_env::Fixed;
use typesafe_sdk_http::{Exchange, Mock};
use typesafe_sdk_retry::RetryPolicy;

const MODELS: &str = r#"{"models":[]}"#;

fn fast_retry(max_retries: u32) -> RetryPolicy {
    RetryPolicy {
        max_retries,
        backoff_initial_ms: 1,
        backoff_max_ms: 1,
        ..RetryPolicy::default()
    }
}

fn client_over(script: Vec<Exchange>, policy: RetryPolicy) -> (Client, Arc<Mock>) {
    let mock = Arc::new(Mock::new(script));
    let config = Builder::new()
        .api_key("k")
        .retry(policy)
        .build(&Fixed::default())
        .unwrap();
    (Client::with_transport(config, mock.clone()), mock)
}

#[tokio::test]
async fn the_request_id_is_reachable_on_success_not_only_on_failure() {
    let script = vec![Exchange::ok(MODELS).with_header("x-typesafe-request-id", "req_abc")];
    let (client, _) = client_over(script, fast_retry(2));

    let responded = client.models_with(&RequestOptions::new()).await.unwrap();
    assert_eq!(responded.request_id.as_deref(), Some("req_abc"));
    assert_eq!(responded.status, 200);
    assert!(responded.data.is_empty());
}

#[tokio::test]
async fn a_response_without_a_request_id_reports_none() {
    let (client, _) = client_over(vec![Exchange::ok(MODELS)], fast_retry(2));
    let responded = client.models_with(&RequestOptions::new()).await.unwrap();
    assert_eq!(responded.request_id, None);
}

/// A per-call policy replaces the client's for that call only.
#[tokio::test]
async fn a_per_call_policy_overrides_the_client_one() {
    let script = vec![
        Exchange::status(503, ""),
        Exchange::status(503, ""),
        Exchange::status(503, ""),
        Exchange::ok(MODELS),
    ];
    let (client, mock) = client_over(script, fast_retry(0));

    let options = RequestOptions::new().retry(fast_retry(3));
    client.models_with(&options).await.unwrap();
    assert_eq!(mock.attempts(), 4, "the per-call policy must win");
}

#[tokio::test]
async fn the_client_policy_survives_a_per_call_override() {
    let script = vec![Exchange::status(503, ""), Exchange::ok(MODELS)];
    let (client, mock) = client_over(script, fast_retry(5));

    let options = RequestOptions::new().retry(fast_retry(0));
    assert!(client.models_with(&options).await.is_err());
    assert_eq!(mock.attempts(), 1);

    // The next call, with no override, uses the client's own policy again.
    assert_eq!(client.config().retry.max_retries, 5);
}

#[tokio::test]
async fn a_per_call_header_is_merged_beneath_the_sdk_headers() {
    let (client, mock) = client_over(vec![Exchange::ok(MODELS)], fast_retry(0));
    let options = RequestOptions::new()
        .header("X-Trace", "abc123")
        .header("authorization", "Bearer stolen");

    client.models_with(&options).await.unwrap();
    let sent = &mock.requests()[0];
    assert_eq!(sent.headers.get("x-trace"), Some("abc123"));
    assert_eq!(sent.headers.get("authorization"), Some("Bearer k"));
}

#[tokio::test]
async fn a_per_call_timeout_reaches_the_transport() {
    let (client, mock) = client_over(vec![Exchange::ok(MODELS)], fast_retry(0));
    let options = RequestOptions::new().timeout_ms(1234);

    client.models_with(&options).await.unwrap();
    assert_eq!(mock.requests()[0].timeout_ms, 1234);
}

#[tokio::test]
async fn the_client_timeout_applies_when_no_override_is_given() {
    let (client, mock) = client_over(vec![Exchange::ok(MODELS)], fast_retry(0));
    client.models().await.unwrap();
    assert_eq!(mock.requests()[0].timeout_ms, 10_000);
}

#[tokio::test]
async fn a_non_retryable_status_is_not_retried_however_many_are_allowed() {
    let (client, mock) = client_over(vec![Exchange::status(400, "")], fast_retry(5));
    let error = client.models().await.unwrap_err();
    assert_eq!(error.status(), Some(400));
    assert_eq!(mock.attempts(), 1);
}

#[tokio::test]
async fn retries_stop_once_the_budget_is_spent() {
    let script = vec![
        Exchange::status(503, ""),
        Exchange::status(503, ""),
        Exchange::status(503, ""),
    ];
    let (client, mock) = client_over(script, fast_retry(2));
    let error = client.models().await.unwrap_err();
    assert_eq!(error.status(), Some(503));
    assert_eq!(mock.attempts(), 3, "one attempt plus two retries");
}
