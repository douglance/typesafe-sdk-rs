//! The question vocabulary: noul, choice and score.
//!
//! A question is plain JSON with a `type` discriminator, and the builders here
//! exist for the checks they perform rather than the objects they return. Two
//! of those checks catch the same mistake from opposite directions: `choice`
//! takes a map of labels and `score` takes an ordered list, and confusing them
//! is the single easiest error to make against this API.

mod build;
mod entry;
mod question;
mod validate;

pub use build::{choice, choice_of, noul, noul_with, questions, score};
pub use entry::Entry;
pub use question::{ChoiceCriteria, NoulCriteria, Question, Questions};
pub use validate::validate;
