//! What a noul answer is worth, against the real service.
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
use typesafe_sdk_questions::{NoulCriteria, noul, noul_with, questions};

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

/// A probability inside 0..1 is not evidence of anything — a parser returning
/// zero for everything would satisfy it. What the contract actually promises is
/// that the answer moves with the text, so this compares two opposite states
/// and asserts the ordering rather than an absolute threshold, which survives
/// recalibration of the model.
#[tokio::test]
async fn a_noul_answer_moves_with_the_text() {
    let client = live_client!();
    let angry = noul_about(
        &client,
        "I was charged twice. This is outrageous, fix it now.",
    )
    .await;
    let calm = noul_about(&client, "Thanks for the quick help yesterday, all sorted.").await;

    assert!(
        angry > calm,
        "an irate message scored {angry} and a grateful one {calm}"
    );
    assert!((0.0..=1.0).contains(&angry) && (0.0..=1.0).contains(&calm));
}

/// The probability that one piece of text has an angry customer behind it.
async fn noul_about(client: &Client, text: &str) -> f64 {
    let request =
        SystemOneRequest::new(text, questions([("angry", noul("Is the customer angry?"))]));
    let response = client
        .system_one(request)
        .await
        .expect("POST /v1/systemone");
    response.expect("angry").unwrap().as_noul().unwrap().noul
}

#[tokio::test]
async fn one_sided_noul_criteria_are_accepted() {
    let client = live_client!();
    let request = SystemOneRequest::new(
        "The build has been broken for three days.",
        questions([(
            "urgent",
            noul_with(
                "Is this urgent?",
                NoulCriteria {
                    yes: Some("blocks other people from working".into()),
                    no: None,
                },
            ),
        )]),
    );
    let response = client.system_one(request).await.expect("one-sided noul");
    let urgent = response.expect("urgent").unwrap().as_noul().unwrap().noul;
    assert!(
        urgent > 0.5,
        "a three-day outage blocking other people scored {urgent}; the criterion \
         describing exactly that case was evidently not applied"
    );
}
