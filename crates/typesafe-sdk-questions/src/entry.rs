//! What may appear as instructions, state, or a criterion description.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Text, a JSON object or array, or `null`.
///
/// The API accepts rich JSON anywhere it accepts a description, so a criterion
/// can carry structure rather than a sentence. This is a thin wrapper over
/// [`Value`] that names that intent and keeps `null` a first-class option.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Entry(pub Value);

impl Entry {
    /// The undescribed entry, which serialises as JSON `null`.
    #[must_use]
    pub const fn null() -> Self {
        Self(Value::Null)
    }

    /// Whether this entry describes nothing.
    #[must_use]
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

impl From<&str> for Entry {
    fn from(text: &str) -> Self {
        Self(Value::String(text.to_owned()))
    }
}

impl From<String> for Entry {
    fn from(text: String) -> Self {
        Self(Value::String(text))
    }
}

impl From<Value> for Entry {
    fn from(value: Value) -> Self {
        Self(value)
    }
}

impl<T: Into<Self>> From<Option<T>> for Entry {
    fn from(value: Option<T>) -> Self {
        value.map_or_else(Self::null, Into::into)
    }
}
