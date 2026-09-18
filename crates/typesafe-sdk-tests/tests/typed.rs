//! The typed question set, from declaration to answer.
//!
//! This is the feature the crate is named for: the labels a caller declares
//! become an enum, so a `match` over the outcomes is exhaustive and a typo in
//! a label is a compile error rather than a runtime surprise.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot fail loudly is not a test"
)]

use typesafe_sdk_answers::SystemOneResponse;
use typesafe_sdk_derive::Questions;

/// A support ticket, as a caller would declare it.
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

/// Captured from the live service for exactly this question set.
const CAPTURED: &str = r#"{
  "model": "jev-1.13.0",
  "answers": {
    "category": {"type":"choice","choice":"billing","confidence":1.0,
                 "probabilities":{"billing":1.0,"technical":0.0,"other":0.0}},
    "urgency":  {"type":"score","score":1.98,"confidence":0.97,
                 "legend":{"0":"can wait","1":"this week","2":"today"},
                 "probabilities":{"0":0.0,"1":0.02,"2":0.98}},
    "angry":    {"type":"noul","noul":0.84}
  },
  "usage": {"input_tokens":358,"output_tokens":70}
}"#;

#[test]
fn the_declared_questions_become_the_wire_format() {
    let set = Ticket::questions();
    let names: Vec<&str> = set.keys().map(String::as_str).collect();
    assert_eq!(names, vec!["category", "urgency", "angry"]);

    let wire = serde_json::to_value(&set).unwrap();
    assert_eq!(wire["category"]["type"], "choice");
    assert_eq!(
        wire["category"]["instructions"],
        "What is this ticket about?"
    );
    assert_eq!(
        wire["category"]["criteria"],
        serde_json::json!({"billing": null, "technical": null, "other": null})
    );
    assert_eq!(
        wire["urgency"]["criteria"],
        serde_json::json!(["can wait", "this week", "today"])
    );
    assert_eq!(wire["angry"]["type"], "noul");
}

#[test]
fn a_choice_answer_arrives_as_an_enum_not_a_string() {
    let response: SystemOneResponse = serde_json::from_str(CAPTURED).unwrap();
    let answers = Ticket::answers(&response).unwrap();

    assert_eq!(answers.category, TicketCategory::Billing);
    // The point of the enum: this match cannot silently miss a case.
    let team = match answers.category {
        TicketCategory::Billing => "finance",
        TicketCategory::Technical => "engineering",
        TicketCategory::Other => "triage",
    };
    assert_eq!(team, "finance");
}

#[test]
fn scores_and_nouls_arrive_as_numbers() {
    let response: SystemOneResponse = serde_json::from_str(CAPTURED).unwrap();
    let answers = Ticket::answers(&response).unwrap();
    assert!((answers.urgency - 1.98).abs() < 1e-9);
    assert!((answers.angry - 0.84).abs() < 1e-9);
}

#[test]
fn every_label_round_trips_through_its_wire_form() {
    for variant in [
        TicketCategory::Billing,
        TicketCategory::Technical,
        TicketCategory::Other,
    ] {
        assert_eq!(TicketCategory::from_label(variant.label()), Some(variant));
    }
    assert_eq!(TicketCategory::from_label("nonexistent"), None);
    assert_eq!(TicketCategory::Billing.to_string(), "billing");
}

#[test]
fn a_missing_answer_is_reported_rather_than_defaulted() {
    let partial = r#"{"model":"m","answers":{},"usage":{"input_tokens":1,"output_tokens":1}}"#;
    let response: SystemOneResponse = serde_json::from_str(partial).unwrap();
    let message = Ticket::answers(&response).unwrap_err().to_string();
    assert!(
        message.contains("No answer named \"category\""),
        "{message}"
    );
}

/// A question asked as a choice but answered as a noul must not be coerced.
#[test]
fn a_mismatched_answer_shape_is_an_error() {
    let mismatched = r#"{"model":"m",
      "answers":{"category":{"type":"noul","noul":0.5},
                 "urgency":{"type":"score","score":1.0,"confidence":1.0,"legend":{},"probabilities":{}},
                 "angry":{"type":"noul","noul":0.1}},
      "usage":{"input_tokens":1,"output_tokens":1}}"#;
    let response: SystemOneResponse = serde_json::from_str(mismatched).unwrap();
    let message = Ticket::answers(&response).unwrap_err().to_string();
    assert!(message.contains("different shape"), "{message}");
}

/// A label the service returns that was never offered must not be invented.
#[test]
fn an_unknown_label_is_an_error_rather_than_a_guess() {
    let surprising = r#"{"model":"m",
      "answers":{"category":{"type":"choice","choice":"refund","confidence":1.0,"probabilities":{}},
                 "urgency":{"type":"score","score":1.0,"confidence":1.0,"legend":{},"probabilities":{}},
                 "angry":{"type":"noul","noul":0.1}},
      "usage":{"input_tokens":1,"output_tokens":1}}"#;
    let response: SystemOneResponse = serde_json::from_str(surprising).unwrap();
    assert!(Ticket::answers(&response).is_err());
}
