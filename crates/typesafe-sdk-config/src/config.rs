//! The resolved settings a client runs on.

use std::sync::Arc;

use typesafe_sdk_headers::Headers;
use typesafe_sdk_log::{Filtered, Level};
use typesafe_sdk_retry::RetryPolicy;

/// Where the API lives when nothing says otherwise.
pub const DEFAULT_BASE_URL: &str = "https://api.typesafe.ai";

/// The model used when a request does not name one.
pub const DEFAULT_MODEL: &str = "jev-latest";

/// Everything a client needs, resolved and checked.
///
/// The API key is deliberately not public and has no accessor returning it by
/// value; it reaches the wire through [`Config::authorization`] and nowhere
/// else, so it cannot be logged or serialised by accident.
pub struct Config {
    api_key: String,
    /// The API root, with trailing slashes stripped.
    pub base_url: String,
    /// The model used when a request does not name one.
    pub default_model: String,
    /// Per-attempt timeout, covering the full response body.
    pub timeout_ms: u64,
    /// When and how often to try again.
    pub retry: RetryPolicy,
    /// Headers applied to every request, beneath the SDK's own.
    pub default_headers: Headers,
    /// The filtered log sink.
    pub logger: Arc<Filtered>,
}

impl Config {
    /// The `Authorization` header value.
    ///
    /// Building the whole header value here means the key itself never leaves
    /// this type.
    #[must_use]
    pub fn authorization(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    /// The last four characters of the key, for diagnostics that must identify
    /// which credential is in use without revealing it.
    #[must_use]
    pub fn key_hint(&self) -> String {
        let count = self.api_key.chars().count();
        if count <= 8 {
            return "***".to_owned();
        }
        let tail: String = self.api_key.chars().skip(count - 4).collect();
        format!("***{tail}")
    }

    /// The level below which log lines are dropped.
    #[must_use]
    pub fn log_level(&self) -> Level {
        self.logger.threshold()
    }

    /// Assembles a config from a key and everything else.
    #[must_use]
    pub fn new(api_key: String, parts: Parts) -> Self {
        Self {
            api_key,
            base_url: parts.base_url,
            default_model: parts.default_model,
            timeout_ms: parts.timeout_ms,
            retry: parts.retry,
            default_headers: parts.default_headers,
            logger: parts.logger,
        }
    }
}

/// Everything except the key, so [`Config::new`] stays within the argument limit.
///
/// Public because the builder lives in a sibling module; it carries no secret.
pub struct Parts {
    /// The API root, already stripped of trailing slashes.
    pub base_url: String,
    /// The model used when a request does not name one.
    pub default_model: String,
    /// Per-attempt timeout.
    pub timeout_ms: u64,
    /// The retry policy.
    pub retry: RetryPolicy,
    /// Headers applied beneath the SDK's own.
    pub default_headers: Headers,
    /// The filtered log sink.
    pub logger: Arc<Filtered>,
}

impl std::fmt::Debug for Config {
    /// Never prints the key. A `Debug` that leaks a credential is how keys end
    /// up in logs and issue reports.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("base_url", &self.base_url)
            .field("default_model", &self.default_model)
            .field("timeout_ms", &self.timeout_ms)
            .field("api_key", &self.key_hint())
            .finish_non_exhaustive()
    }
}
