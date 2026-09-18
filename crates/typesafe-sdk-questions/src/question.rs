//! The three question shapes, as they go on the wire.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::entry::Entry;

/// Descriptions of the yes and no outcomes of a noul question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct NoulCriteria {
    /// Description of the yes outcome.
    #[serde(rename = "true", skip_serializing_if = "Option::is_none")]
    pub yes: Option<Entry>,
    /// Description of the no outcome.
    #[serde(rename = "false", skip_serializing_if = "Option::is_none")]
    pub no: Option<Entry>,
}

/// Labels mapped to descriptions. Order is preserved, as the API echoes it back.
pub type ChoiceCriteria = IndexMap<String, Entry>;

/// One question, identified by its `type` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Question {
    /// A yes/no question, answered with a probability.
    Noul {
        /// The question itself.
        #[serde(skip_serializing_if = "Option::is_none")]
        instructions: Option<Entry>,
        /// Optional descriptions of either outcome.
        #[serde(skip_serializing_if = "Option::is_none")]
        criteria: Option<NoulCriteria>,
    },
    /// A selection between named alternatives.
    Choice {
        /// The question itself.
        #[serde(skip_serializing_if = "Option::is_none")]
        instructions: Option<Entry>,
        /// The labels available, mapped to their descriptions.
        criteria: ChoiceCriteria,
    },
    /// A score against an ordered rubric, indexed from zero.
    Score {
        /// The question itself.
        #[serde(skip_serializing_if = "Option::is_none")]
        instructions: Option<Entry>,
        /// At least two descriptions, indexed by score from zero.
        criteria: Vec<Entry>,
    },
}

impl Question {
    /// The wire discriminator, for messages that name it.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Noul { .. } => "noul",
            Self::Choice { .. } => "choice",
            Self::Score { .. } => "score",
        }
    }
}

/// Questions keyed by the names their answers will carry.
///
/// Insertion-ordered so a request reads the way it was written, and so the
/// serialised body is stable between runs.
pub type Questions = IndexMap<String, Question>;
