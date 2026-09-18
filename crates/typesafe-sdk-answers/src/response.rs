//! What comes back from `POST /v1/systemone`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use typesafe_sdk_error::{Error, Result};

use crate::answer::Answer;

/// Answers keyed by the question names that were asked.
pub type Answers = IndexMap<String, Answer>;

/// Tokens consumed by one request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    /// Input tokens used.
    pub input_tokens: u64,
    /// Output tokens used.
    pub output_tokens: u64,
}

/// A System One response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemOneResponse {
    /// The model that answered.
    ///
    /// This is the resolved version rather than the alias that was requested:
    /// asking for `jev-latest` answers with something like `jev-1.13.0`.
    pub model: String,
    /// The answers, keyed by question name.
    pub answers: Answers,
    /// Tokens consumed.
    pub usage: Usage,
}

impl SystemOneResponse {
    /// The answer to one question.
    #[must_use]
    pub fn answer(&self, name: &str) -> Option<&Answer> {
        self.answers.get(name)
    }

    /// The answer to one question, or an error naming what was missing.
    ///
    /// # Errors
    /// Returns [`Error::Invalid`] when no answer carries that name.
    pub fn expect(&self, name: &str) -> Result<&Answer> {
        self.answers.get(name).ok_or_else(|| {
            let asked = self
                .answers
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            Error::Invalid(format!(
                "No answer named \"{name}\" in the response; got: {asked}."
            ))
        })
    }
}
