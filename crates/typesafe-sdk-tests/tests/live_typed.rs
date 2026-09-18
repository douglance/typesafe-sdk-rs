//! The typed question set against the real service.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot fail loudly is not a test"
)]

use std::sync::Arc;

use typesafe_sdk_client::{Client, SystemOneRequest};
use typesafe_sdk_config::Builder;
use typesafe_sdk_derive::Questions;
use typesafe_sdk_env::Process;
use typesafe_sdk_http::Reqwest;

/// A support ticket, declared once and used for both asking and reading.
#[derive(Questions)]
struct Ticket {
    /// What is this ticket about?
    #[choice(billing, technical, other)]
    category: (),
    /// How urgent is it?
    #[score("can wait", "this week", "today")]
    urgency: (),
    /// Is the customer angry?
    #[noul]
    angry: (),
}

fn live() -> Option<Client> {
    if std::env::var("TYPESAFE_API_KEY").is_err() {
        eprintln!("skipping: TYPESAFE_API_KEY is not set");
        return None;
    }
    let config = Builder::new().build(&Process).ok()?;
    Some(Client::with_transport(
        config,
        Arc::new(Reqwest::new().ok()?),
    ))
}

/// The declared questions are accepted by the service, and its answers read
/// back through the generated types without any hand-written parsing.
#[tokio::test]
async fn a_declared_question_set_round_trips_through_the_service() {
    let Some(client) = live() else { return };

    let request = SystemOneRequest::new(
        "I was charged twice. Please fix this ASAP.",
        Ticket::questions(),
    );
    let response = client
        .system_one(request)
        .await
        .expect("POST /v1/systemone");
    let answers = Ticket::answers(&response).expect("typed answers");

    assert_eq!(answers.category, TicketCategory::Billing);
    assert!((0.0..=2.0).contains(&answers.urgency));
    assert!((0.0..=1.0).contains(&answers.angry));
}

/// The same declaration, over text that should land on a different label.
#[tokio::test]
async fn a_different_ticket_selects_a_different_label() {
    let Some(client) = live() else { return };

    let request = SystemOneRequest::new(
        "The API returns a 500 whenever I upload a file larger than 2MB.",
        Ticket::questions(),
    );
    let response = client
        .system_one(request)
        .await
        .expect("POST /v1/systemone");
    let answers = Ticket::answers(&response).expect("typed answers");

    assert_eq!(
        answers.category,
        TicketCategory::Technical,
        "a 500 on upload is a technical ticket"
    );
}
