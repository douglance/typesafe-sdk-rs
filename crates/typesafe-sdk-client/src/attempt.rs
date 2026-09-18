//! Running one request until it succeeds, fails, or runs out of attempts.

use typesafe_sdk_config::Config;
use typesafe_sdk_error::{Body, Error, Result};
use typesafe_sdk_http::{RawResponse, Request, Transport};
use typesafe_sdk_log::Level;
use typesafe_sdk_retry::{Wait, delay_ms};

/// How one attempt ended.
enum Outcome {
    /// Finished, either with a response to return or an error to raise.
    Done(Result<RawResponse>),
    /// Worth trying again; carries the delay and why.
    Again { delay: u64, reason: String },
}

/// Sends `build(attempt)` until it succeeds or the policy gives up.
///
/// # Errors
/// Returns the last error when every attempt is exhausted.
pub(crate) async fn run(
    transport: &dyn Transport,
    config: &Config,
    tag: &str,
    build: impl Fn(u32) -> Request,
) -> Result<RawResponse> {
    let mut last: Result<RawResponse> = Err(Error::Invalid("no attempt was made".to_owned()));

    for attempt in 0..=config.retry.max_retries {
        let sent = transport.send(build(attempt)).await;
        match classify(config, attempt, sent) {
            Outcome::Done(result) => return result,
            Outcome::Again { delay, reason } => {
                announce(
                    config,
                    tag,
                    &Retrying {
                        attempt,
                        delay,
                        reason: &reason,
                    },
                );
                last = Err(Error::Invalid(reason));
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }
        }
    }
    last
}

/// One decision to try again.
struct Retrying<'a> {
    attempt: u32,
    delay: u64,
    reason: &'a str,
}

/// Reports that the request will be tried again, and why.
fn announce(config: &Config, tag: &str, retrying: &Retrying<'_>) {
    let Retrying {
        attempt,
        delay,
        reason,
    } = *retrying;
    config.logger.log(Level::Info, || {
        format!(
            "{tag} retrying in {delay}ms (retry {}/{}) after {reason}",
            attempt + 1,
            config.retry.max_retries
        )
    });
}

/// Decides whether an attempt's result ends the request or earns another try.
fn classify(config: &Config, attempt: u32, sent: Result<RawResponse>) -> Outcome {
    let exhausted = attempt >= config.retry.max_retries;
    match sent {
        Ok(response) => from_response(config, attempt, exhausted, response),
        Err(error) => from_error(config, attempt, exhausted, error),
    }
}

/// A response arrived; its status decides.
fn from_response(config: &Config, attempt: u32, exhausted: bool, response: RawResponse) -> Outcome {
    if response.is_success() {
        return Outcome::Done(Ok(response));
    }
    if exhausted || !config.retry.retries_status(response.status) {
        return Outcome::Done(Err(failure(&response)));
    }
    Outcome::Again {
        delay: wait_for(config, attempt, Some(&response)),
        reason: response.status.to_string(),
    }
}

/// Nothing arrived; the failure decides.
fn from_error(config: &Config, attempt: u32, exhausted: bool, error: Error) -> Outcome {
    if exhausted || !config.retry.retries_error(&error) {
        return Outcome::Done(Err(error));
    }
    let reason = error.to_string();
    Outcome::Again {
        delay: wait_for(config, attempt, None),
        reason,
    }
}

/// Builds the error for a non-2xx response.
fn failure(response: &RawResponse) -> Error {
    Error::from_response(
        response.status,
        Body::parse(&response.body),
        &response.headers,
    )
}

fn wait_for(config: &Config, attempt: u32, response: Option<&RawResponse>) -> u64 {
    delay_ms(&Wait {
        attempt,
        headers: response.map(|r| &r.headers),
        policy: &config.retry,
        random: &rand_unit,
        now_ms: now_ms(),
    })
}

/// A cheap uniform value in `[0, 1)`.
///
/// Jitter only has to decorrelate concurrent clients, so the system clock's low
/// bits are sufficient and avoid a dependency for it.
fn rand_unit() -> f64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.subsec_nanos());
    f64::from(nanos % 1_000_000) / 1_000_000.0
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|since| i64::try_from(since.as_millis()).ok())
        .unwrap_or(0)
}
