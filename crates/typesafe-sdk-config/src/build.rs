//! Resolving settings from arguments, the environment and the defaults.

use std::sync::Arc;

use typesafe_sdk_env::{Source, Var, from_code_or_env, read};
use typesafe_sdk_error::{Error, Result};
use typesafe_sdk_headers::Headers;
use typesafe_sdk_log::{Console, Filtered, Level, Logger};
use typesafe_sdk_retry::{DEFAULT_TIMEOUT_MS, RetryPolicy};

use crate::config::{Config, DEFAULT_BASE_URL, DEFAULT_MODEL, Parts};

/// Collects explicit settings before they are resolved against the environment.
#[derive(Default)]
pub struct Builder {
    api_key: Option<String>,
    base_url: Option<String>,
    default_model: Option<String>,
    log_level: Option<Level>,
    timeout_ms: Option<u64>,
    retry: Option<RetryPolicy>,
    default_headers: Headers,
    logger: Option<Arc<dyn Logger>>,
}

impl Builder {
    /// A builder with nothing set, so every setting comes from the environment.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the API key explicitly.
    #[must_use]
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Overrides the API root.
    #[must_use]
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Overrides the model used when a request does not name one.
    #[must_use]
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = Some(model.into());
        self
    }

    /// Overrides the log level.
    #[must_use]
    pub const fn log_level(mut self, level: Level) -> Self {
        self.log_level = Some(level);
        self
    }

    /// Overrides the per-attempt timeout.
    #[must_use]
    pub const fn timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = Some(ms);
        self
    }

    /// Overrides the retry policy.
    #[must_use]
    pub fn retry(mut self, policy: RetryPolicy) -> Self {
        self.retry = Some(policy);
        self
    }

    /// Adds a header sent with every request, beneath the SDK's own.
    #[must_use]
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.default_headers.apply(name, value);
        self
    }

    /// Replaces the log sink.
    #[must_use]
    pub fn logger(mut self, logger: Arc<dyn Logger>) -> Self {
        self.logger = Some(logger);
        self
    }

    /// Resolves and validates every setting.
    ///
    /// # Errors
    /// Returns [`Error::Invalid`] when the key is missing or a setting cannot
    /// mean anything sensible.
    pub fn build(self, env: &impl Source) -> Result<Config> {
        let api_key = from_code_or_env(self.api_key, env, Var::ApiKey).ok_or_else(missing_key)?;
        let retry = self.retry.unwrap_or_default();
        retry.validate()?;
        let timeout_ms = validated_timeout(self.timeout_ms)?;

        Ok(Config::new(
            api_key,
            Parts {
                base_url: resolve_base_url(self.base_url, env),
                default_model: from_code_or_env(self.default_model, env, Var::DefaultModel)
                    .unwrap_or_else(|| DEFAULT_MODEL.to_owned()),
                timeout_ms,
                retry,
                default_headers: self.default_headers,
                logger: Arc::new(Filtered::new(
                    self.logger.unwrap_or_else(|| Arc::new(Console)),
                    resolve_level(self.log_level, env)?,
                )),
            },
        ))
    }
}

fn missing_key() -> Error {
    Error::Invalid(format!(
        "No API key was provided. Pass `api_key` to the client builder or set the {} \
         environment variable.",
        Var::ApiKey
    ))
}

/// A timeout of zero would time out before the request began.
fn validated_timeout(requested: Option<u64>) -> Result<u64> {
    match requested {
        None => Ok(DEFAULT_TIMEOUT_MS),
        Some(0) => Err(Error::Invalid(
            "`timeout` must be a positive number of milliseconds, got 0.".to_owned(),
        )),
        Some(ms) => Ok(ms),
    }
}

/// Trailing slashes are stripped so paths concatenate without doubling.
fn resolve_base_url(from_code: Option<String>, env: &impl Source) -> String {
    let url = from_code_or_env(from_code, env, Var::BaseUrl)
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned());
    url.trim_end_matches('/').to_owned()
}

fn resolve_level(from_code: Option<Level>, env: &impl Source) -> Result<Level> {
    if let Some(level) = from_code {
        return Ok(level);
    }
    read(env, Var::LogLevel).map_or_else(
        || Ok(typesafe_sdk_log::DEFAULT_LEVEL),
        |raw| Level::parse(&raw, Var::LogLevel.name()),
    )
}
