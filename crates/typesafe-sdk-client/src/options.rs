//! Per-call overrides, and what a response carried besides its body.

use typesafe_sdk_headers::Headers;
use typesafe_sdk_retry::RetryPolicy;

/// Settings that apply to one call only.
///
/// Omitted fields inherit the client's. There is no per-call base URL or API
/// key: those identify *which* service and *whose* account, and varying them
/// per request is a sign that two clients are wanted rather than one.
#[derive(Debug, Clone, Default)]
pub struct RequestOptions {
    /// Timeout for each attempt. There is no total budget across retries.
    pub timeout_ms: Option<u64>,
    /// Retry policy for this call.
    pub retry: Option<RetryPolicy>,
    /// Headers merged over the client's defaults, beneath the SDK's own.
    pub headers: Headers,
}

impl RequestOptions {
    /// Options that change nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Overrides the per-attempt timeout for this call.
    #[must_use]
    pub const fn timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = Some(ms);
        self
    }

    /// Overrides the retry policy for this call.
    #[must_use]
    pub fn retry(mut self, policy: RetryPolicy) -> Self {
        self.retry = Some(policy);
        self
    }

    /// Adds a header to this call only.
    #[must_use]
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.apply(name, value);
        self
    }
}

/// A result together with what the response carried alongside it.
///
/// The request id is what support needs to find one call in the service's logs,
/// so it must be reachable on success and not only on failure.
#[derive(Debug, Clone)]
pub struct Responded<T> {
    /// The parsed result.
    pub data: T,
    /// The HTTP status.
    pub status: u16,
    /// Value of `x-typesafe-request-id`, when the service supplied one.
    pub request_id: Option<String>,
}

impl<T> Responded<T> {
    /// Discards the metadata, keeping the result.
    #[must_use]
    pub fn into_data(self) -> T {
        self.data
    }
}
