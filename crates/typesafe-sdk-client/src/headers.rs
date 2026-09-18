//! Assembling the headers for one attempt.
//!
//! The order is the whole point: the caller's defaults go on first, then the
//! per-request headers, then the SDK's own. Because the SDK writes last, a
//! caller cannot replace `Authorization` with their own value, attach a
//! `Content-Type` to a request that has no body, or forge the retry count —
//! whatever casing they try, since the map compares case-insensitively.

use typesafe_sdk_config::Config;
use typesafe_sdk_headers::{Headers, Value};

use crate::VERSION;

/// Identifies the SDK in `User-Agent` and `X-TypeSafe-SDK`.
fn agent() -> String {
    format!("typesafe-sdk/{VERSION}")
}

/// Builds the headers for one attempt of one request.
///
/// `attempt` is zero-based; the retry count header is absent on the first.
pub(crate) fn assemble(
    config: &Config,
    per_request: &Headers,
    has_body: bool,
    attempt: u32,
) -> Headers {
    let mut headers = config.default_headers.clone();
    for (name, value) in per_request {
        headers.apply(name, value);
    }
    apply_sdk_headers(&mut headers, config, has_body, attempt);
    headers
}

/// The headers the SDK owns, applied last so they always win.
fn apply_sdk_headers(headers: &mut Headers, config: &Config, has_body: bool, attempt: u32) {
    headers.apply("Authorization", config.authorization());
    headers.apply("Accept", "application/json");
    headers.apply("User-Agent", agent());
    headers.apply("X-TypeSafe-SDK", agent());
    headers.apply("X-TypeSafe-Runtime", typesafe_sdk_runtime::describe());
    headers.apply(
        "Content-Type",
        if has_body {
            Value::Set("application/json".to_owned())
        } else {
            Value::Remove
        },
    );
    headers.apply(
        "X-TypeSafe-Retry-Count",
        if attempt == 0 {
            Value::Remove
        } else {
            Value::Set(attempt.to_string())
        },
    );
}
