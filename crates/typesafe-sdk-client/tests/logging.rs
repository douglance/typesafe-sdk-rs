//! What reaches a log, and what must never.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot fail loudly is not a test"
)]

use std::sync::Arc;

use typesafe_sdk_client::Client;
use typesafe_sdk_config::Builder;
use typesafe_sdk_env::Fixed;
use typesafe_sdk_http::{Exchange, Mock};
use typesafe_sdk_log::{Level, Recording};

const KEY: &str = "apikey_0123456789abcdef";
const MODELS: &str = r#"{"models":[]}"#;

fn client_at(level: Level) -> (Client, Arc<Recording>) {
    let log = Arc::new(Recording::default());
    let config = Builder::new()
        .api_key(KEY)
        .log_level(level)
        .logger(log.clone())
        .build(&Fixed::default())
        .unwrap();
    let mock = Arc::new(Mock::new(vec![Exchange::ok(MODELS)]));
    (Client::with_transport(config, mock), log)
}

fn transcript(log: &Recording) -> String {
    log.lines()
        .iter()
        .map(|(level, line)| format!("{level} {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The one property that matters most: a key must not reach a log, at any
/// level, however verbose.
#[tokio::test]
async fn the_api_key_never_reaches_the_log() {
    let (client, log) = client_at(Level::Debug);
    client.models().await.unwrap();

    let written = transcript(&log);
    assert!(!written.is_empty(), "debug logging produced nothing");
    assert!(!written.contains(KEY), "the key leaked:\n{written}");
    assert!(
        !written.contains("0123456789"),
        "the secret leaked:\n{written}"
    );
}

/// Redacted rather than omitted: knowing the scheme is what makes an auth
/// failure diagnosable.
#[tokio::test]
async fn the_authorization_header_is_masked_but_still_visible() {
    let (client, log) = client_at(Level::Debug);
    client.models().await.unwrap();

    let written = transcript(&log);
    assert!(
        written.contains("Authorization: Bearer ***cdef"),
        "expected a masked Authorization header:\n{written}"
    );
}

/// A healthy client at the default level says nothing at all.
#[tokio::test]
async fn the_default_level_is_silent_on_success() {
    let (client, log) = client_at(Level::Warn);
    client.models().await.unwrap();
    assert_eq!(transcript(&log), "");
}

#[tokio::test]
async fn info_reports_one_line_per_request() {
    let (client, log) = client_at(Level::Info);
    client.models().await.unwrap();

    let written = transcript(&log);
    assert!(written.contains("GET /v1/models <- 200"), "{written}");
}

/// The SDK returns failures to the caller rather than logging them itself, so
/// it never writes at warn or error.
#[tokio::test]
async fn the_sdk_never_logs_at_warn_or_error() {
    let log = Arc::new(Recording::default());
    let config = Builder::new()
        .api_key(KEY)
        .log_level(Level::Debug)
        .logger(log.clone())
        .build(&Fixed::default())
        .unwrap();
    let mock = Arc::new(Mock::new(vec![Exchange::status(400, "")]));
    let client = Client::with_transport(config, mock);

    assert!(client.models().await.is_err());
    for (level, line) in log.lines() {
        assert!(level < Level::Warn, "the SDK wrote at {level}: {line}");
    }
}
