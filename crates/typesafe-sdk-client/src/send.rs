//! Turning a call into attempts.

use typesafe_sdk_config::Config;
use typesafe_sdk_error::Result;
use typesafe_sdk_http::{Method, RawResponse, Request, Transport};
use typesafe_sdk_log::Level;
use typesafe_sdk_retry::RetryPolicy;

use crate::attempt::{self, Plan};
use crate::headers::assemble;
use crate::options::RequestOptions;

/// One call, before it becomes attempts.
pub(crate) struct Call<'a> {
    /// The method.
    pub(crate) method: Method,
    /// The path beneath the base URL.
    pub(crate) path: &'a str,
    /// The serialised body, when there is one.
    pub(crate) body: Option<String>,
    /// Per-call overrides.
    pub(crate) options: &'a RequestOptions,
}

/// Sends `call`, retrying under the effective policy.
///
/// # Errors
/// Returns the last error when every attempt is exhausted.
pub(crate) async fn send(
    transport: &dyn Transport,
    config: &Config,
    call: &Call<'_>,
) -> Result<RawResponse> {
    let url = format!("{}{}", config.base_url, call.path);
    let tag = format!("{} {}", call.method, call.path);
    let policy = effective_policy(config, call.options);
    let timeout_ms = call.options.timeout_ms.unwrap_or(config.timeout_ms);

    config
        .logger
        .log(Level::Debug, || format!("{tag} -> {url}"));

    let plan = Plan {
        config,
        policy: &policy,
        tag: &tag,
    };
    attempt::run(transport, &plan, |attempt| Request {
        method: call.method,
        url: url.clone(),
        headers: assemble(config, &call.options.headers, call.body.is_some(), attempt),
        body: call.body.clone(),
        timeout_ms,
    })
    .await
}

/// The policy for this call: the per-call one when given, the client's otherwise.
fn effective_policy(config: &Config, options: &RequestOptions) -> RetryPolicy {
    options
        .retry
        .clone()
        .unwrap_or_else(|| config.retry.clone())
}
