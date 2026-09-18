//! Conformance against the real service.
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

use typesafe_sdk_answers::SystemOneResponse;
use typesafe_sdk_client::{Client, SystemOneRequest};
use typesafe_sdk_config::Builder;
use typesafe_sdk_env::Process;
use typesafe_sdk_http::Reqwest;
use typesafe_sdk_questions::{NoulCriteria, choice_of, noul, noul_with, questions, score};

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
async fn the_models_listing_returns_real_cards() {
    let client = live_client!();
    let models = client.models().await.expect("GET /v1/models");

    assert!(
        !models.is_empty(),
        "the account must see at least one model"
    );
    let latest = models
        .iter()
        .find(|card| card.name == "jev-latest")
        .expect("jev-latest must be listed");
    assert!(!latest.description.is_empty());
    assert!(
        latest.release_date.starts_with("20"),
        "unexpected release date {:?}",
        latest.release_date
    );
}

/// The ticket every assertion below is about.
const TICKET: &str = "I was charged twice. Please fix this ASAP.";

/// Asks all three kinds at once, as a caller would.
async fn ask_about_the_ticket(client: &Client) -> SystemOneResponse {
    let request = SystemOneRequest::new(
        TICKET,
        questions([
            (
                "category",
                choice_of(
                    "What is this ticket about?",
                    ["billing", "technical", "other"],
                ),
            ),
            (
                "urgency",
                score("How urgent is it?", ["can wait", "this week", "today"]),
            ),
            ("angry", noul("Is the customer angry?")),
        ]),
    );
    client
        .system_one(request)
        .await
        .expect("POST /v1/systemone")
}

#[tokio::test]
async fn the_response_reports_the_resolved_model_and_real_usage() {
    let client = live_client!();
    let response = ask_about_the_ticket(&client).await;
    assert!(
        response.model.starts_with("jev-"),
        "unexpected model {:?}",
        response.model
    );
    assert!(response.usage.input_tokens > 0);
}

#[tokio::test]
async fn a_choice_question_selects_a_label_with_calibrated_probabilities() {
    let client = live_client!();
    let response = ask_about_the_ticket(&client).await;
    let category = response.expect("category").unwrap().as_choice().unwrap();

    assert_eq!(category.choice, "billing", "a double charge is billing");
    assert_eq!(category.probabilities.len(), 3);
    let total: f64 = category.probabilities.values().sum();
    assert!(
        (total - 1.0).abs() < 1e-6,
        "probabilities summed to {total}"
    );
}

#[tokio::test]
async fn a_score_question_answers_within_its_rubric_and_echoes_it_back() {
    let client = live_client!();
    let response = ask_about_the_ticket(&client).await;
    let urgency = response.expect("urgency").unwrap().as_score().unwrap();

    assert!(
        (0.0..=2.0).contains(&urgency.score),
        "score {} outside the rubric",
        urgency.score
    );
    assert_eq!(
        urgency.describe(2).and_then(|v| v.as_str()),
        Some("today"),
        "the rubric must come back as sent"
    );
}

#[tokio::test]
async fn a_noul_question_answers_with_a_probability() {
    let client = live_client!();
    let response = ask_about_the_ticket(&client).await;
    let angry = response.expect("angry").unwrap().as_noul().unwrap();
    assert!((0.0..=1.0).contains(&angry.noul));
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
    assert!(response.expect("urgent").unwrap().as_noul().is_some());
}
