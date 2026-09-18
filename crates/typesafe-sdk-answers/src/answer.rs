//! One answer, in each of the three shapes.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A yes/no answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoulAnswer {
    /// Probability of a yes answer, from zero to one.
    pub noul: f64,
}

/// A selected label, with the probability of every label.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChoiceAnswer {
    /// The selected label.
    pub choice: String,
    /// Reported confidence in the selection.
    pub confidence: f64,
    /// Probability of each label, keyed by label.
    pub probabilities: IndexMap<String, f64>,
}

/// A score against the rubric, with the rubric echoed back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreAnswer {
    /// Expected score. Falls between rubric levels when the model is unsure.
    pub score: f64,
    /// Reported confidence in the score.
    pub confidence: f64,
    /// The rubric, keyed by score as a decimal string.
    pub legend: IndexMap<String, serde_json::Value>,
    /// Probability of each score, keyed by score as a decimal string.
    pub probabilities: IndexMap<String, f64>,
}

/// One answer, identified by the `type` it echoes from the question.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Answer {
    /// The answer to a noul question.
    Noul(NoulAnswer),
    /// The answer to a choice question.
    Choice(ChoiceAnswer),
    /// The answer to a score question.
    Score(ScoreAnswer),
}

impl Answer {
    /// The wire discriminator.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Noul(_) => "noul",
            Self::Choice(_) => "choice",
            Self::Score(_) => "score",
        }
    }

    /// The noul answer, if this is one.
    #[must_use]
    pub const fn as_noul(&self) -> Option<&NoulAnswer> {
        match self {
            Self::Noul(answer) => Some(answer),
            _ => None,
        }
    }

    /// The choice answer, if this is one.
    #[must_use]
    pub const fn as_choice(&self) -> Option<&ChoiceAnswer> {
        match self {
            Self::Choice(answer) => Some(answer),
            _ => None,
        }
    }

    /// The score answer, if this is one.
    #[must_use]
    pub const fn as_score(&self) -> Option<&ScoreAnswer> {
        match self {
            Self::Score(answer) => Some(answer),
            _ => None,
        }
    }
}

impl ScoreAnswer {
    /// The rubric description for an integer score, when the legend has one.
    #[must_use]
    pub fn describe(&self, score: usize) -> Option<&serde_json::Value> {
        self.legend.get(&score.to_string())
    }
}

impl ChoiceAnswer {
    /// The probability the service gave one label.
    #[must_use]
    pub fn probability_of(&self, label: &str) -> Option<f64> {
        self.probabilities.get(label).copied()
    }
}
