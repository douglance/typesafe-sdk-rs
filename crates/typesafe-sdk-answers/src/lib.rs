//! Answers, usage and the System One response.
//!
//! The service answers each question with an object carrying the same `type`
//! discriminator the question had. Accessors are fallible rather than panicking
//! so a mismatch between what was asked and what came back is a value a caller
//! can handle, not a crash.

mod answer;
mod response;

pub use answer::{Answer, ChoiceAnswer, NoulAnswer, ScoreAnswer};
pub use response::{Answers, SystemOneResponse, Usage};
