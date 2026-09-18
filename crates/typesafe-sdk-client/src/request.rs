//! The System One request body.
//!
//! The JavaScript SDK spreads the caller's whole object onto the wire, so a
//! field it does not model still reaches the service. That is worth copying:
//! dropping unknown fields would make this SDK the reason a new API option
//! could not be used until someone cut a release.

use indexmap::IndexMap;
use serde::Serialize;
use serde_json::Value;
use typesafe_sdk_questions::{Entry, Questions};

/// State and questions for one System One call.
///
/// `extra` exists because the JavaScript SDK spreads the caller's whole request
/// object onto the wire, so a field the SDK does not model still reaches the
/// service. Dropping unknown fields would make this SDK the reason a new API
/// option could not be used.
#[derive(Debug, Clone, Serialize)]
pub struct SystemOneRequest {
    /// What the questions are about.
    pub state: Entry,
    /// The questions, keyed by the names their answers will carry.
    pub questions: Questions,
    /// The model to use; filled from the client default when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Any further fields, forwarded verbatim.
    #[serde(flatten)]
    pub extra: IndexMap<String, Value>,
}

impl SystemOneRequest {
    /// A request asking `questions` about `state`.
    #[must_use]
    pub fn new(state: impl Into<Entry>, questions: Questions) -> Self {
        Self {
            state: state.into(),
            questions,
            model: None,
            extra: IndexMap::new(),
        }
    }

    /// Overrides the model for this request only.
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Forwards a field the SDK does not model.
    #[must_use]
    pub fn extra(mut self, name: impl Into<String>, value: Value) -> Self {
        self.extra.insert(name.into(), value);
        self
    }

    /// Fills the model from the client default when the request did not name one.
    pub(crate) fn resolve_model(&mut self, default: &str) {
        if self.model.is_none() {
            self.model = Some(default.to_owned());
        }
    }
}
