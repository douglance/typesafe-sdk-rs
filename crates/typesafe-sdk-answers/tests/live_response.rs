//! Parsing a response captured verbatim from the live API.
//!
//! The body below is the exact one `api.typesafe.ai` returned for a request
//! carrying all three question kinds. Parsing it is the only check that proves
//! the types describe the service rather than describing each other.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot fail loudly is not a test"
)]

use typesafe_sdk_answers::SystemOneResponse;

/// Captured from `POST /v1/systemone`, request id `req_01a0b556…`.
const CAPTURED: &str = r#"{
  "model": "jev-1.13.0",
  "answers": {
    "category": {"type":"choice","choice":"billing","confidence":1.0,
                 "probabilities":{"technical":0.0,"other":0.0,"billing":1.0}},
    "urgency":  {"type":"score","score":1.98,"confidence":0.97,
                 "legend":{"0":"can wait","1":"this week","2":"today"},
                 "probabilities":{"0":0.0,"1":0.02,"2":0.98}},
    "angry":    {"type":"noul","noul":0.84}
  },
  "usage": {"input_tokens":358,"output_tokens":70}
}"#;

fn captured() -> SystemOneResponse {
    serde_json::from_str(CAPTURED).expect("the captured body must parse")
}

#[test]
fn the_resolved_model_is_reported_not_the_alias_requested() {
    assert_eq!(captured().model, "jev-1.13.0");
}

#[test]
fn a_choice_answer_carries_its_label_and_every_probability() {
    let response = captured();
    let choice = response.expect("category").unwrap().as_choice().unwrap();
    assert_eq!(choice.choice, "billing");
    assert!((choice.confidence - 1.0).abs() < f64::EPSILON);
    assert_eq!(choice.probability_of("billing"), Some(1.0));
    assert_eq!(choice.probability_of("technical"), Some(0.0));
    assert_eq!(choice.probability_of("absent"), None);
}

/// The score is an expectation, so it falls between rubric levels.
#[test]
fn a_score_answer_may_be_fractional() {
    let response = captured();
    let score = response.expect("urgency").unwrap().as_score().unwrap();
    assert!((score.score - 1.98).abs() < 1e-9);
    assert_eq!(score.describe(2).and_then(|v| v.as_str()), Some("today"));
    assert_eq!(score.describe(9), None);
}

#[test]
fn the_rubric_comes_back_keyed_by_score_as_a_string() {
    let response = captured();
    let score = response.expect("urgency").unwrap().as_score().unwrap();
    let keys: Vec<&str> = score.legend.keys().map(String::as_str).collect();
    assert_eq!(keys, vec!["0", "1", "2"]);
}

#[test]
fn probabilities_sum_to_one() {
    let response = captured();
    for name in ["category", "urgency"] {
        let answer = response.expect(name).unwrap();
        let total: f64 = match answer {
            typesafe_sdk_answers::Answer::Choice(c) => c.probabilities.values().sum(),
            typesafe_sdk_answers::Answer::Score(s) => s.probabilities.values().sum(),
            typesafe_sdk_answers::Answer::Noul(_) => 1.0,
        };
        assert!((total - 1.0).abs() < 1e-6, "{name} summed to {total}");
    }
}

#[test]
fn a_noul_answer_is_a_probability() {
    let response = captured();
    let noul = response.expect("angry").unwrap().as_noul().unwrap();
    assert!((noul.noul - 0.84).abs() < 1e-9);
}

#[test]
fn asking_for_the_wrong_shape_yields_none_rather_than_panicking() {
    let response = captured();
    assert!(response.expect("angry").unwrap().as_choice().is_none());
    assert_eq!(response.expect("angry").unwrap().kind(), "noul");
}

#[test]
fn a_missing_answer_names_what_did_come_back() {
    let message = captured().expect("nonexistent").unwrap_err().to_string();
    assert_eq!(
        message,
        "No answer named \"nonexistent\" in the response; got: category, urgency, angry."
    );
}

#[test]
fn usage_is_reported_for_the_request() {
    let usage = captured().usage;
    assert_eq!((usage.input_tokens, usage.output_tokens), (358, 70));
}

#[test]
fn answers_keep_the_order_the_questions_were_asked_in() {
    let response = captured();
    let names: Vec<&str> = response.answers.keys().map(String::as_str).collect();
    assert_eq!(names, vec!["category", "urgency", "angry"]);
}
