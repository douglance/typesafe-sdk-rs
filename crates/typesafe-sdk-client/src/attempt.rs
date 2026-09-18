//! Running one request until it succeeds, fails, or runs out of attempts.

use typesafe_sdk_config::Config;
use typesafe_sdk_error::{Body, Error, Result};
use typesafe_sdk_http::{RawResponse, Request, Transport};
use typesafe_sdk_log::Level;
use typesafe_sdk_retry::{RetryPolicy, Wait, delay_ms};

/// How one attempt ended.
enum Outcome {
    /// Finished, either with a response to return or an error to raise.
    Done(Result<RawResponse>),
    /// Worth trying again; carries the delay and why.
    Again {
        /// Milliseconds to wait first.
        delay: u64,
        /// What went wrong, for the log line.
        reason: String,
    },
}

/// What one request is being sent under.
pub(crate) struct Plan<'a> {
    /// The client's settings, for logging.
    pub(crate) config: &'a Config,
    /// The policy in force, which may be a per-call override.
    pub(crate) policy: &'a RetryPolicy,
    /// How this request is named in the log.
    pub(crate) tag: &'a str,
}

/// Where one attempt sits: its policy, its number, and whether it is the last.
struct At<'a> {
    policy: &'a RetryPolicy,
    attempt: u32,
    exhausted: bool,
}

/// Sends `build(attempt)` until it succeeds or the policy gives up.
///
/// # Errors
/// Returns the last error when every attempt is exhausted.
pub(crate) async fn run(
    transport: &dyn Transport,
    plan: &Plan<'_>,
    build: impl Fn(u32) -> Request,
) -> Result<RawResponse> {
    let mut last: Result<RawResponse> = Err(Error::Invalid("no attempt was made".to_owned()));

    for attempt in 0..=plan.policy.max_retries {
        let at = At {
            policy: plan.policy,
            attempt,
            exhausted: attempt >= plan.policy.max_retries,
        };
        match classify(&at, transport.send(build(attempt)).await) {
            Outcome::Done(result) => return result,
            Outcome::Again { delay, reason } => {
                announce(
                    plan,
                    &at,
                    &Report {
                        delay,
                        reason: &reason,
                    },
                );
                last = Err(Error::Invalid(reason));
                pause(delay).await;
            }
        }
    }
    last
}

/// Waits before the next attempt.
async fn pause(delay: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
}

/// What to say about a retry.
struct Report<'a> {
    delay: u64,
    reason: &'a str,
}

/// Reports that the request will be tried again, and why.
fn announce(plan: &Plan<'_>, at: &At<'_>, report: &Report<'_>) {
    let Report { delay, reason } = *report;
    let tag = plan.tag;
    plan.config.logger.log(Level::Info, || {
        format!(
            "{tag} retrying in {delay}ms (retry {}/{}) after {reason}",
            at.attempt + 1,
            at.policy.max_retries
        )
    });
}

/// Decides whether an attempt's result ends the request or earns another try.
fn classify(at: &At<'_>, sent: Result<RawResponse>) -> Outcome {
    match sent {
        Ok(response) => from_response(at, response),
        Err(error) => from_error(at, error),
    }
}

/// A response arrived; its status decides.
fn from_response(at: &At<'_>, response: RawResponse) -> Outcome {
    if response.is_success() {
        return Outcome::Done(Ok(response));
    }
    if at.exhausted || !at.policy.retries_status(response.status) {
        return Outcome::Done(Err(failure(&response)));
    }
    Outcome::Again {
        delay: wait_for(at, Some(&response)),
        reason: response.status.to_string(),
    }
}

/// Nothing arrived; the failure decides.
fn from_error(at: &At<'_>, error: Error) -> Outcome {
    if at.exhausted || !at.policy.retries_error(&error) {
        return Outcome::Done(Err(error));
    }
    let reason = error.to_string();
    Outcome::Again {
        delay: wait_for(at, None),
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

fn wait_for(at: &At<'_>, response: Option<&RawResponse>) -> u64 {
    delay_ms(&Wait {
        attempt: at.attempt,
        headers: response.map(|r| &r.headers),
        policy: at.policy,
        random: &rand_unit,
        now_ms: now_ms(),
    })
}

/// A cheap uniform value in `[0, 1)`.
///
/// Jitter only has to decorrelate concurrent clients, so the system clock's low
/// bits are enough and avoid taking a dependency for it.
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
