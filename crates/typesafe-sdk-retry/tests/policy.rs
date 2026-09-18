//! The retry contract, ported case for case from the JavaScript suite.

use typesafe_sdk_error::Error;
use typesafe_sdk_headers::Headers;
use typesafe_sdk_retry::{RetryPolicy, delay_ms};

/// 2026-09-18T00:00:00Z, computed independently of the code under test.
const NOW_MS: i64 = 1_789_689_600_000;

fn headers(pairs: &[(&str, &str)]) -> Headers {
    pairs
        .iter()
        .fold(Headers::new(), |acc, &(n, v)| acc.with(n, v))
}

fn delay(attempt: u32, policy: &RetryPolicy, random: f64) -> u64 {
    delay_ms(attempt, None, policy, &|| random, NOW_MS)
}

#[test]
fn the_defaults_are_the_documented_ones() {
    let policy = RetryPolicy::default();
    assert_eq!(policy.max_retries, 2);
    assert_eq!(policy.backoff_initial_ms, 500);
    assert_eq!(policy.backoff_max_ms, 5_000);
    assert!((policy.backoff_jitter - 0.25).abs() < f64::EPSILON);
    assert_eq!(policy.max_retry_after_ms, 60_000);
    assert!(policy.respect_retry_after);
    assert!(policy.api_connection_error);
    assert!(policy.api_timeout_error);
}

#[test]
fn the_default_status_set_is_exactly_408_429_and_5xx() {
    let policy = RetryPolicy::default();
    let expected: Vec<u16> = std::iter::once(408)
        .chain(std::iter::once(429))
        .chain(500..600)
        .collect();
    let actual: Vec<u16> = policy.http_statuses.iter().copied().collect();
    assert_eq!(actual, expected);
}

#[test]
fn retryable_statuses_match_the_table() {
    let policy = RetryPolicy::default();
    for status in [408, 429, 500, 502, 503, 504, 529, 599] {
        assert!(policy.retries_status(status), "{status} should retry");
    }
    for status in [200, 400, 401, 403, 404, 409, 422, 600] {
        assert!(!policy.retries_status(status), "{status} should not retry");
    }
}

#[test]
fn backoff_doubles_to_its_cap() {
    let policy = RetryPolicy::default();
    let growth: Vec<u64> = (0..6).map(|n| delay(n, &policy, 0.0)).collect();
    assert_eq!(growth, vec![500, 1_000, 2_000, 4_000, 5_000, 5_000]);
}

/// Jitter subtracts. Full jitter on the first attempt is 500 - 25%.
#[test]
fn jitter_only_ever_subtracts() {
    let policy = RetryPolicy::default();
    assert_eq!(delay(0, &policy, 1.0), 375);
    assert_eq!(delay(1, &policy, 0.5), 875);
    for attempt in 0..6 {
        let nominal = delay(attempt, &policy, 0.0);
        assert!(delay(attempt, &policy, 1.0) <= nominal);
    }
}

#[test]
fn an_accepted_retry_after_is_honoured_exactly_with_no_jitter() {
    let policy = RetryPolicy::default();
    let h = headers(&[("retry-after", "2")]);
    assert_eq!(delay_ms(0, Some(&h), &policy, &|| 1.0, NOW_MS), 2_000);
}

/// Sixty seconds is the boundary and is inclusive; sixty-one falls back.
#[test]
fn the_retry_after_cap_is_inclusive() {
    let policy = RetryPolicy::default();
    let honoured = headers(&[("retry-after", "60")]);
    assert_eq!(
        delay_ms(0, Some(&honoured), &policy, &|| 0.0, NOW_MS),
        60_000
    );

    let refused = headers(&[("retry-after", "61")]);
    assert_eq!(delay_ms(0, Some(&refused), &policy, &|| 0.0, NOW_MS), 500);
}

#[test]
fn the_cap_applies_to_the_millisecond_header_too() {
    let policy = RetryPolicy::default();
    let honoured = headers(&[("retry-after-ms", "60000")]);
    assert_eq!(
        delay_ms(0, Some(&honoured), &policy, &|| 0.0, NOW_MS),
        60_000
    );

    let refused = headers(&[("retry-after-ms", "60001")]);
    assert_eq!(delay_ms(0, Some(&refused), &policy, &|| 0.0, NOW_MS), 500);
}

#[test]
fn disabling_retry_after_ignores_the_server() {
    let policy = RetryPolicy {
        respect_retry_after: false,
        ..RetryPolicy::default()
    };
    let h = headers(&[("retry-after", "2")]);
    assert_eq!(delay_ms(0, Some(&h), &policy, &|| 0.0, NOW_MS), 500);
}

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
        http_statuses: [42].into_iter().collect(),
        ..RetryPolicy::default()
    };
    let message = bad_status.validate().unwrap_err().to_string();
    assert!(message.contains("retry.httpStatuses"), "{message}");
}
