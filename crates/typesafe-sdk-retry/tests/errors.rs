//! Which failures the policy tries again, and which it never does.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot fail loudly is not a test"
)]

use typesafe_sdk_error::Error;
use typesafe_sdk_retry::RetryPolicy;
#[test]
fn a_caller_abort_is_never_retried() {
    let policy = RetryPolicy::default();
    assert!(!policy.retries_error(&Error::aborted()));
    assert!(policy.retries_error(&Error::connection("reset")));
    assert!(policy.retries_error(&Error::Timeout { timeout_ms: 10_000 }));
}

/// Timeouts and connection failures are configured independently, even though
/// a timeout is also a connection failure.
#[test]
fn timeouts_and_connection_failures_are_separable() {
    let no_connection = RetryPolicy {
        api_connection_error: false,
        ..RetryPolicy::default()
    };
    assert!(!no_connection.retries_error(&Error::connection("reset")));
    assert!(no_connection.retries_error(&Error::Timeout { timeout_ms: 1 }));

    let no_timeout = RetryPolicy {
        api_timeout_error: false,
        ..RetryPolicy::default()
    };
    assert!(no_timeout.retries_error(&Error::connection("reset")));
    assert!(!no_timeout.retries_error(&Error::Timeout { timeout_ms: 1 }));
}

#[test]
fn an_empty_status_set_disables_response_retrying() {
    let policy = RetryPolicy {
        http_statuses: std::collections::BTreeSet::new(),
        ..RetryPolicy::default()
    };
    assert!(!policy.retries_status(503));
}

#[test]
fn an_impossible_policy_is_rejected_by_field_name() {
    let bad_jitter = RetryPolicy {
        backoff_jitter: 1.5,
        ..RetryPolicy::default()
    };
    let message = bad_jitter.validate().unwrap_err().to_string();
    assert!(message.contains("retry.backoffJitter"), "{message}");

    let bad_status = RetryPolicy {
        http_statuses: std::iter::once(42).collect(),
        ..RetryPolicy::default()
    };
    let message = bad_status.validate().unwrap_err().to_string();
    assert!(message.contains("retry.httpStatuses"), "{message}");
}
