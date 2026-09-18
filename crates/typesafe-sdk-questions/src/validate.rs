//! What the client refuses to send.
//!
//! These checks run before any request, so a malformed question costs nothing
//! and reports in the caller's terms rather than as a server validation list.
//! The client is deliberately stricter than the API here: a single-entry score
//! rubric is accepted by the service but cannot express a comparison, so it is
//! rejected up front, exactly as the JavaScript SDK rejects it.

use typesafe_sdk_error::{Error, Result};

use crate::question::{Question, Questions};

/// The fewest rubric entries a score question can be answered against.
const MIN_SCORE_CRITERIA: usize = 2;

/// Checks a question set before it is sent.
///
/// # Errors
/// Returns [`Error::Invalid`] naming the offending question.
pub fn validate(questions: &Questions) -> Result<()> {
    if questions.is_empty() {
        return Err(Error::Invalid(
            "At least one question is required.".to_owned(),
        ));
    }
    questions
        .iter()
        .try_for_each(|(name, question)| check_one(name, question))
}

fn check_one(name: &str, question: &Question) -> Result<()> {
    let Question::Score { criteria, .. } = question else {
        return Ok(());
    };
    if criteria.len() < MIN_SCORE_CRITERIA {
        return Err(Error::Invalid(format!(
            "Score question \"{name}\" has {} criteria; at least two scores are required.",
            criteria.len()
        )));
    }
    Ok(())
}
