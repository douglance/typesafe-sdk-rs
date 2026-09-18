//! The retry contract, ported case for case from the JavaScript suite.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot fail loudly is not a test"
)]

use typesafe_sdk_headers::Headers;
use typesafe_sdk_retry::{RetryPolicy, Wait, delay_ms};

/// 2026-09-18T00:00:00Z, computed independently of the code under test.
const NOW_MS: i64 = 1_789_689_600_000;

fn headers(pairs: &[(&str, &str)]) -> Headers {
    pairs
        .iter()
        .fold(Headers::new(), |acc, &(n, v)| acc.with(n, v))
}

fn delay(attempt: u32, policy: &RetryPolicy, random: f64) -> u64 {
    wait_for(attempt, None, policy, random)
}

fn wait_for(attempt: u32, headers: Option<&Headers>, policy: &RetryPolicy, random: f64) -> u64 {
    delay_ms(&Wait {
        attempt,
        headers,
        policy,
        random: &|| random,
        now_ms: NOW_MS,
    })
}

#[test]
fn the_default_delays_are_the_documented_ones() {
    let policy = RetryPolicy::default();
    assert_eq!(
        (
            policy.max_retries,
            policy.backoff_initial_ms,
            policy.backoff_max_ms,
            policy.max_retry_after_ms
        ),
        (2, 500, 5_000, 60_000)
    );
    assert!((policy.backoff_jitter - 0.25).abs() < f64::EPSILON);
}

#[test]
fn every_default_switch_is_on() {
    let policy = RetryPolicy::default();
    assert_eq!(
        (
            policy.respect_retry_after,
            policy.api_connection_error,
            policy.api_timeout_error
        ),
        (true, true, true)
    );
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
    assert_eq!(wait_for(0, Some(&h), &policy, 1.0), 2_000);
}

/// Sixty seconds is the boundary and is inclusive; sixty-one falls back.
#[test]
fn the_retry_after_cap_is_inclusive() {
    let policy = RetryPolicy::default();
    let honoured = headers(&[("retry-after", "60")]);
    assert_eq!(wait_for(0, Some(&honoured), &policy, 0.0), 60_000);

    let refused = headers(&[("retry-after", "61")]);
    assert_eq!(wait_for(0, Some(&refused), &policy, 0.0), 500);
}

#[test]
fn the_cap_applies_to_the_millisecond_header_too() {
    let policy = RetryPolicy::default();
    let honoured = headers(&[("retry-after-ms", "60000")]);
    assert_eq!(wait_for(0, Some(&honoured), &policy, 0.0), 60_000);

    let refused = headers(&[("retry-after-ms", "60001")]);
    assert_eq!(wait_for(0, Some(&refused), &policy, 0.0), 500);
}

#[test]
fn disabling_retry_after_ignores_the_server() {
    let policy = RetryPolicy {
        respect_retry_after: false,
        ..RetryPolicy::default()
    };
    let h = headers(&[("retry-after", "2")]);
    assert_eq!(wait_for(0, Some(&h), &policy, 0.0), 500);
}
