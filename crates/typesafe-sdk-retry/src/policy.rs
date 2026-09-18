//! What the client retries, and how patiently.

use std::collections::BTreeSet;

use typesafe_sdk_error::{Error, Result};

/// Per-attempt timeout. Covers the full response body, not just the headers.
pub const DEFAULT_TIMEOUT_MS: u64 = 10_000;

/// Retries after the first attempt, so three attempts in total by default.
pub const DEFAULT_MAX_RETRIES: u32 = 2;

/// When the client tries again, and how long it waits.
#[derive(Debug, Clone, PartialEq)]
pub struct RetryPolicy {
    /// Attempts after the first. Zero disables retrying.
    pub max_retries: u32,
    /// Delay before the first retry, doubling from there.
    pub backoff_initial_ms: u64,
    /// Ceiling on the exponential delay.
    pub backoff_max_ms: u64,
    /// Fraction of the delay that jitter may subtract, from 0.0 to 1.0.
    pub backoff_jitter: f64,
    /// Response statuses worth trying again.
    pub http_statuses: BTreeSet<u16>,
    /// Whether to honour a server's `Retry-After`.
    pub respect_retry_after: bool,
    /// Longest server-requested delay accepted before falling back to backoff.
    pub max_retry_after_ms: u64,
    /// Whether a connection failure is retried.
    pub api_connection_error: bool,
    /// Whether a timeout is retried.
    pub api_timeout_error: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        let mut http_statuses: BTreeSet<u16> = (500..600).collect();
        http_statuses.insert(408);
        http_statuses.insert(429);
        Self {
            max_retries: DEFAULT_MAX_RETRIES,
            backoff_initial_ms: 500,
            backoff_max_ms: 5_000,
            backoff_jitter: 0.25,
            http_statuses,
            respect_retry_after: true,
            max_retry_after_ms: 60_000,
            api_connection_error: true,
            api_timeout_error: true,
        }
    }
}

impl RetryPolicy {
    /// Whether this policy tries `status` again.
    #[must_use]
    pub fn retries_status(&self, status: u16) -> bool {
        self.http_statuses.contains(&status)
    }

    /// Whether this policy tries `error` again.
    ///
    /// A caller's cancellation is never retried; a timeout is checked before a
    /// general connection failure, because the two are configured separately
    /// even though a timeout is also a connection failure.
    #[must_use]
    pub fn retries_error(&self, error: &Error) -> bool {
        match error {
            Error::Timeout { .. } => self.api_timeout_error,
            Error::Connection { .. } => self.api_connection_error,
            Error::Api(api) => self.retries_status(api.status),
            // A cancelled request has no result to wait for, and a rejected
            // one never reached the network. Neither improves on a second try.
            Error::UserAbort { .. } | Error::Invalid(_) => false,
        }
    }

    /// Rejects a policy that cannot mean anything sensible.
    ///
    /// # Errors
    /// Returns [`Error::Invalid`] naming the offending field.
    pub fn validate(&self) -> Result<()> {
        if !(0.0..=1.0).contains(&self.backoff_jitter) || !self.backoff_jitter.is_finite() {
            return Err(Error::Invalid(format!(
                "`retry.backoffJitter` must be between 0 and 1, got {}.",
                self.backoff_jitter
            )));
        }
        if let Some(bad) = self
            .http_statuses
            .iter()
            .find(|&&s| !(100..=999).contains(&s))
        {
            return Err(Error::Invalid(format!(
                "`retry.httpStatuses` must contain integers between 100 and 999, got {bad}."
            )));
        }
        Ok(())
    }
}
