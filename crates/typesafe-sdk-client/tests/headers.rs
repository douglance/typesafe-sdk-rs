//! What actually goes on the wire, and what a caller cannot change.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot fail loudly is not a test"
)]

use std::sync::Arc;

use typesafe_sdk_client::{Client, SystemOneRequest, VERSION};
use typesafe_sdk_config::Builder;
use typesafe_sdk_env::Fixed;
use typesafe_sdk_http::{Exchange, Mock, Request};
use typesafe_sdk_questions::{choice_of, questions};

const MODELS: &str = r#"{"models":[]}"#;

fn client_over(
    script: Vec<Exchange>,
    configure: impl FnOnce(Builder) -> Builder,
) -> (Client, Arc<Mock>) {
    let mock = Arc::new(Mock::new(script));
    let config = configure(Builder::new().api_key("sk_live_0123456789abcdef"))
        .build(&Fixed::default())
        .unwrap();
    (Client::with_transport(config, mock.clone()), mock)
}

fn header<'a>(request: &'a Request, name: &str) -> Option<&'a str> {
    request.headers.get(name)
}

#[tokio::test]
async fn a_get_carries_the_sdk_headers_and_no_content_type() {
    let (client, mock) = client_over(vec![Exchange::ok(MODELS)], |b| b);
    client.models().await.unwrap();

    let sent = &mock.requests()[0];
    assert_eq!(
        header(sent, "authorization"),
        Some("Bearer sk_live_0123456789abcdef")
    );
    assert_eq!(header(sent, "accept"), Some("application/json"));
    let agent = format!("typesafe-sdk/{VERSION}");
    assert_eq!(header(sent, "user-agent"), Some(agent.as_str()));
    assert_eq!(header(sent, "x-typesafe-sdk"), Some(agent.as_str()));
    assert!(
        header(sent, "x-typesafe-runtime")
            .unwrap()
            .starts_with("rust/")
    );
    assert_eq!(header(sent, "content-type"), None, "a GET has no body");
    assert_eq!(header(sent, "x-typesafe-retry-count"), None);
    assert_eq!(sent.url, "https://api.typesafe.ai/v1/models");
}

#[tokio::test]
async fn a_post_declares_json_and_targets_system_one() {
    let body =
        r#"{"model":"jev-1.13.0","answers":{},"usage":{"input_tokens":1,"output_tokens":1}}"#;
    let (client, mock) = client_over(vec![Exchange::ok(body)], |b| b);
    let request = SystemOneRequest::new("hi", questions([("q", choice_of("Which?", ["a", "b"]))]));
    client.system_one(request).await.unwrap();

    let sent = &mock.requests()[0];
    assert_eq!(header(sent, "content-type"), Some("application/json"));
    assert_eq!(sent.url, "https://api.typesafe.ai/v1/systemone");
    assert!(
        sent.body
            .as_ref()
            .unwrap()
            .contains(r#""model":"jev-latest""#)
    );
}

/// The SDK writes its headers last, so a caller cannot replace them whatever
/// casing they use.
#[tokio::test]
async fn a_caller_cannot_clobber_the_sdk_headers() {
    let (client, mock) = client_over(vec![Exchange::ok(MODELS)], |b| {
        b.header("AUTHORIZATION", "Bearer stolen")
            .header("User-Agent", "not-the-sdk")
            .header("x-typesafe-retry-count", "99")
            .header("X-Team", "platform")
    });
    client.models().await.unwrap();

    let sent = &mock.requests()[0];
    assert_eq!(
        header(sent, "authorization"),
        Some("Bearer sk_live_0123456789abcdef")
    );
    assert_eq!(
        header(sent, "user-agent"),
        Some(format!("typesafe-sdk/{VERSION}").as_str())
    );
    assert_eq!(header(sent, "x-typesafe-retry-count"), None);
    assert_eq!(
        header(sent, "x-team"),
        Some("platform"),
        "own headers survive"
    );
}

#[tokio::test]
async fn only_one_authorization_header_reaches_the_wire() {
    let (client, mock) = client_over(vec![Exchange::ok(MODELS)], |b| {
        b.header("authorization", "Bearer a")
    });
    client.models().await.unwrap();
    let count = mock.requests()[0]
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("authorization"))
        .count();
    assert_eq!(count, 1);
}

/// Absent on the first attempt, then counting up.
#[tokio::test]
async fn the_retry_count_header_appears_only_on_retries() {
    let script = vec![
        Exchange::status(503, ""),
        Exchange::status(503, ""),
        Exchange::ok(MODELS),
    ];
    let (client, mock) = client_over(script, |b| {
        b.retry(typesafe_sdk_retry::RetryPolicy {
            backoff_initial_ms: 1,
            ..typesafe_sdk_retry::RetryPolicy::default()
        })
    });
    client.models().await.unwrap();

    let sent = mock.requests();
    assert_eq!(sent.len(), 3);
    assert_eq!(header(&sent[0], "x-typesafe-retry-count"), None);
    assert_eq!(header(&sent[1], "x-typesafe-retry-count"), Some("1"));
    assert_eq!(header(&sent[2], "x-typesafe-retry-count"), Some("2"));
}
