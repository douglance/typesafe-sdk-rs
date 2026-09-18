//! How long to wait before the next attempt.
//!
//! Two properties are easy to get subtly wrong and are pinned by tests. Jitter
//! only ever *subtracts*, so a delay never exceeds its nominal backoff and a
//! thundering herd spreads backwards into the gap rather than outwards past it.
//! And an accepted `Retry-After` is honoured exactly, with no jitter at all,
//! because the server named a time and second-guessing it helps nobody.

use typesafe_sdk_headers::{Headers, parse_retry_after};

use crate::policy::RetryPolicy;

/// A source of randomness in `[0, 1)`, injected so delays are testable.
pub type Random<'a> = &'a dyn Fn() -> f64;

/// Everything the delay for one attempt depends on.
pub struct Wait<'a> {
    /// Zero-based retry number.
    pub attempt: u32,
    /// Response headers, when the attempt failed with a response.
    pub headers: Option<&'a Headers>,
    /// The policy in force for this request.
    pub policy: &'a RetryPolicy,
    /// Randomness for jitter.
    pub random: Random<'a>,
    /// Unix epoch milliseconds, for resolving a `Retry-After` date.
    pub now_ms: i64,
}

/// Milliseconds to wait before the next attempt.
#[must_use]
pub fn delay_ms(wait: &Wait<'_>) -> u64 {
    server_delay(wait.headers, wait.policy, wait.now_ms)
        .unwrap_or_else(|| backoff(wait.attempt, wait.policy, wait.random))
}

/// The server's own delay, when it asked for one the policy will accept.
fn server_delay(headers: Option<&Headers>, policy: &RetryPolicy, now_ms: i64) -> Option<u64> {
    if !policy.respect_retry_after {
        return None;
    }
    let asked = parse_retry_after(headers?, now_ms)?;
    (asked <= policy.max_retry_after_ms).then_some(asked)
}

/// Capped exponential backoff with subtractive jitter.
fn backoff(attempt: u32, policy: &RetryPolicy, random: Random<'_>) -> u64 {
    let doubled = policy
        .backoff_initial_ms
        .checked_shl(attempt)
        .unwrap_or(u64::MAX);
    let exponential = doubled.min(policy.backoff_max_ms);
    // Reads as `1 - r * jitter`; written fused so the rounding is exact.
    let scale = random()
        .clamp(0.0, 1.0)
        .mul_add(-policy.backoff_jitter, 1.0);
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "delays are milliseconds well inside f64's exact integer range"
    )]
    let jittered = (exponential as f64 * scale).round() as u64;
    jittered
}
