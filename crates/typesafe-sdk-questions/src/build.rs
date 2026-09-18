//! Builders that make the easy mistakes impossible to express.

use indexmap::IndexMap;

use crate::entry::Entry;
use crate::question::{ChoiceCriteria, NoulCriteria, Question};

/// A yes/no question.
#[must_use]
pub fn noul(instructions: impl Into<Entry>) -> Question {
    Question::Noul {
        instructions: Some(instructions.into()),
        criteria: None,
    }
}

/// A yes/no question describing one or both outcomes.
#[must_use]
pub fn noul_with(instructions: impl Into<Entry>, criteria: NoulCriteria) -> Question {
    Question::Noul {
        instructions: Some(instructions.into()),
        criteria: Some(criteria),
    }
}

/// A selection between named alternatives.
///
/// Taking labels as pairs rather than a list is what stops a caller passing the
/// ordered rubric a score question wants; the two are not interchangeable and
/// the API rejects the confusion with a 422.
#[must_use]
pub fn choice<K, V>(
    instructions: impl Into<Entry>,
    criteria: impl IntoIterator<Item = (K, V)>,
) -> Question
where
    K: Into<String>,
    V: Into<Entry>,
{
    let criteria: ChoiceCriteria = criteria
        .into_iter()
        .map(|(label, description)| (label.into(), description.into()))
        .collect();
    Question::Choice {
        instructions: Some(instructions.into()),
        criteria,
    }
}

/// A selection between labels with no descriptions.
#[must_use]
pub fn choice_of<K: Into<String>>(
    instructions: impl Into<Entry>,
    labels: impl IntoIterator<Item = K>,
) -> Question {
    let criteria: ChoiceCriteria = labels
        .into_iter()
        .map(|label| (label.into(), Entry::null()))
        .collect();
    Question::Choice {
        instructions: Some(instructions.into()),
        criteria,
    }
}

/// A score against an ordered rubric, indexed from zero.
#[must_use]
pub fn score<V: Into<Entry>>(
    instructions: impl Into<Entry>,
    criteria: impl IntoIterator<Item = V>,
) -> Question {
    Question::Score {
        instructions: Some(instructions.into()),
        criteria: criteria.into_iter().map(Into::into).collect(),
    }
}

/// Assembles a question set in the order given.
#[must_use]
pub fn questions<K: Into<String>>(
    pairs: impl IntoIterator<Item = (K, Question)>,
) -> IndexMap<String, Question> {
    pairs
        .into_iter()
        .map(|(name, question)| (name.into(), question))
        .collect()
}
