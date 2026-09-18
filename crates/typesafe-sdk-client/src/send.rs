//! Turning a call into attempts.

use typesafe_sdk_config::Config;
use typesafe_sdk_error::Result;
use typesafe_sdk_headers::{Headers, redact};
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
    let response = attempt::run(transport, &plan, |attempt| {
        let headers = assemble(config, &call.options.headers, call.body.is_some(), attempt);
        config.logger.log(Level::Debug, || {
            format!("{tag} -> headers {}", safe(&headers))
        });
        Request {
            method: call.method,
            url: url.clone(),
            headers,
            body: call.body.clone(),
            timeout_ms,
        }
    })
    .await?;

    trace(config, &tag, &response);
    Ok(response)
}

/// Records the request and response in full, with credentials masked.
///
/// Header values are redacted rather than omitted, because knowing that an
/// `Authorization` header was present and which scheme it used is exactly what
/// makes an auth failure diagnosable. Bodies are logged unredacted: they are
/// the caller's own text, and masking them would make the log useless.
fn trace(config: &Config, tag: &str, response: &RawResponse) {
    config.logger.log(Level::Debug, || {
        format!("{tag} <- {} {}", response.status, response.body)
    });
}

/// Renders headers for a log line, with credentials masked.
fn safe(headers: &Headers) -> String {
    headers
        .iter()
        .map(|(name, value)| format!("{name}: {}", redact(name, value)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The policy for this call: the per-call one when given, the client's otherwise.
fn effective_policy(config: &Config, options: &RequestOptions) -> RetryPolicy {
    options
        .retry
        .clone()
        .unwrap_or_else(|| config.retry.clone())
}
