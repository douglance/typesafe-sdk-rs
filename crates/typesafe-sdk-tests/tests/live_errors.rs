//! How the real service's failures reach a caller.
//!
//! These are the only tests that can prove the SDK talks to TypeSafe rather
//! than to its own assumptions. They are skipped when `TYPESAFE_API_KEY` is
//! unset, so an ordinary `cargo test` stays offline and deterministic; CI and
//! release checks must run them with a key present.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot fail loudly is not a test"
)]

use std::sync::Arc;

use typesafe_sdk_client::{Client, SystemOneRequest};
use typesafe_sdk_config::Builder;
use typesafe_sdk_env::Process;
use typesafe_sdk_http::Reqwest;
use typesafe_sdk_questions::{noul, questions, score};

/// A client against the real API, or `None` when no key is configured.
fn live() -> Option<Client> {
    if std::env::var("TYPESAFE_API_KEY").is_err() {
        eprintln!("skipping: TYPESAFE_API_KEY is not set");
        return None;
    }
    let config = Builder::new().build(&Process).ok()?;
    let transport = Arc::new(Reqwest::new().ok()?);
    Some(Client::with_transport(config, transport))
}

macro_rules! live_client {
    () => {
        match live() {
            Some(client) => client,
            None => return,
        }
    };
}

#[tokio::test]
async fn an_unknown_model_is_reported_as_a_bad_request() {
    let client = live_client!();
    let request =
        SystemOneRequest::new("hi", questions([("q", noul("Urgent?"))])).model("no-such-model");

    let error = client.system_one(request).await.unwrap_err();
    assert_eq!(error.status(), Some(400));
    assert_eq!(error.to_string(), "400 Unknown model: no-such-model");
}

#[tokio::test]
async fn a_bad_key_is_reported_as_an_authentication_failure() {
    if std::env::var("TYPESAFE_API_KEY").is_err() {
        return;
    }
    let config = Builder::new()
        .api_key("apikey_not_a_real_key")
        .build(&Process)
        .unwrap();
    let client = Client::with_transport(config, Arc::new(Reqwest::new().unwrap()));

    let error = client.models().await.unwrap_err();
    assert_eq!(error.status(), Some(401));
    assert!(
        error.to_string().starts_with("401 Cannot authenticate"),
        "{error}"
    );
}

/// A field the SDK does not model must still reach the service.
#[tokio::test]
async fn unmodelled_request_fields_are_forwarded() {
    let client = live_client!();
    let request = SystemOneRequest::new("hi", questions([("q", noul("Urgent?"))]))
        .extra("future_option", serde_json::Value::Null);

    let response = client.system_one(request).await.expect("forwarded field");
    assert!(response.expect("q").unwrap().as_noul().is_some());
}

/// The client refuses this before any request; the service would accept it.
#[tokio::test]
async fn a_single_entry_rubric_never_reaches_the_network() {
    let client = live_client!();
    let request = SystemOneRequest::new("hi", questions([("q", score("How urgent?", ["only"]))]));

    let error = client.system_one(request).await.unwrap_err();
    assert_eq!(error.status(), None, "must not be an HTTP error");
    assert_eq!(
        error.to_string(),
        "Score question \"q\" has 1 criteria; at least two scores are required."
    );
}
