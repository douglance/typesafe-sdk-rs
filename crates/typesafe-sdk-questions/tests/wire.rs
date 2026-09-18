//! The exact JSON the builders produce.
//!
//! These are byte-level assertions because the wire format is the contract.
//! The same payloads were sent to the live service and accepted, so the shapes
//! here are known-good rather than merely self-consistent.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot fail loudly is not a test"
)]

use serde_json::json;
use typesafe_sdk_questions::{
    Entry, NoulCriteria, Question, choice, choice_of, noul, noul_with, questions, score, validate,
};

fn wire(question: &Question) -> serde_json::Value {
    serde_json::to_value(question).unwrap()
}

#[test]
fn a_noul_question_carries_only_its_instructions() {
    assert_eq!(
        wire(&noul("Is the customer angry?")),
        json!({"type": "noul", "instructions": "Is the customer angry?"})
    );
}

#[test]
fn noul_criteria_describe_either_side_independently() {
    let only_yes = noul_with(
        "Is it urgent?",
        NoulCriteria {
            yes: Some("needs attention today".into()),
            no: None,
        },
    );
    assert_eq!(
        wire(&only_yes),
        json!({
            "type": "noul",
            "instructions": "Is it urgent?",
            "criteria": {"true": "needs attention today"}
        })
    );
}

#[test]
fn choice_labels_map_to_descriptions_and_keep_their_order() {
    let question = choice_of(
        "What is this ticket about?",
        ["billing", "technical", "other"],
    );
    assert_eq!(
        wire(&question),
        json!({
            "type": "choice",
            "instructions": "What is this ticket about?",
            "criteria": {"billing": null, "technical": null, "other": null}
        })
    );
    let rendered = serde_json::to_string(&question).unwrap();
    let billing = rendered.find("billing").unwrap();
    let other = rendered.find("other").unwrap();
    assert!(billing < other, "label order must survive: {rendered}");
}

#[test]
fn a_described_choice_keeps_its_descriptions() {
    let question = choice(
        "Which team?",
        [("billing", "money"), ("technical", "the product")],
    );
    assert_eq!(
        wire(&question),
        json!({
            "type": "choice",
            "instructions": "Which team?",
            "criteria": {"billing": "money", "technical": "the product"}
        })
    );
}

/// The rubric is an ordered list, not a map keyed by number. This is the v0.6.0
/// breaking change, and sending the old shape is a 422.
#[test]
fn a_score_rubric_is_an_ordered_list() {
    assert_eq!(
        wire(&score("How urgent?", ["can wait", "this week", "today"])),
        json!({
            "type": "score",
            "instructions": "How urgent?",
            "criteria": ["can wait", "this week", "today"]
        })
    );
}

#[test]
fn a_description_may_be_rich_json_rather_than_a_sentence() {
    let question = choice(
        "Which tier?",
        [(
            "enterprise",
            Entry::from(json!({"seats": {"min": 500}, "sla": "24h"})),
        )],
    );
    assert_eq!(
        wire(&question)["criteria"]["enterprise"],
        json!({"seats": {"min": 500}, "sla": "24h"})
    );
}

#[test]
fn an_undescribed_label_is_null_not_absent() {
    let rendered = serde_json::to_string(&choice_of("Which?", ["a"])).unwrap();
    assert!(rendered.contains(r#""a":null"#), "{rendered}");
}

#[test]
fn an_empty_question_set_is_refused_before_any_request() {
    let message = validate(&questions(Vec::<(String, Question)>::new()))
        .unwrap_err()
        .to_string();
    assert_eq!(message, "At least one question is required.");
}

/// The service accepts a one-entry rubric; the client does not, because a
/// single score cannot express a comparison.
#[test]
fn a_single_entry_rubric_is_refused_although_the_service_allows_it() {
    let set = questions([("q", score("How urgent?", ["only"]))]);
    let message = validate(&set).unwrap_err().to_string();
    assert_eq!(
        message,
        "Score question \"q\" has 1 criteria; at least two scores are required."
    );
}

#[test]
fn a_two_entry_rubric_is_the_smallest_accepted() {
    let set = questions([("q", score("How urgent?", ["no", "yes"]))]);
    assert!(validate(&set).is_ok());
}

#[test]
fn a_question_set_keeps_the_order_it_was_written_in() {
    let set = questions([
        ("category", choice_of("What?", ["a", "b"])),
        ("urgency", score("How urgent?", ["low", "high"])),
        ("angry", noul("Angry?")),
    ]);
    let names: Vec<&str> = set.keys().map(String::as_str).collect();
    assert_eq!(names, vec!["category", "urgency", "angry"]);
}
