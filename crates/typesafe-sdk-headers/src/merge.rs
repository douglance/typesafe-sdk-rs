//! The header map itself.

use indexmap::IndexMap;

/// What a source says about one header: a value, or its removal.
///
/// `Value::Remove` exists because "unset this" and "set this to empty" are
/// different instructions, and the SDK relies on the difference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// Set the header to this value.
    Set(String),
    /// Remove the header if an earlier source set it.
    Remove,
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::Set(value.to_owned())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Set(value)
    }
}

/// Headers keyed case-insensitively, preserving insertion order and the
/// spelling of whichever source wrote last.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Headers {
    entries: IndexMap<String, (String, String)>,
}

impl Headers {
    /// An empty map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies one source over this map, last value winning.
    pub fn apply(&mut self, name: &str, value: impl Into<Value>) {
        let key = name.to_lowercase();
        match value.into() {
            Value::Set(v) => {
                self.entries.insert(key, (name.to_owned(), v));
            }
            Value::Remove => {
                self.entries.shift_remove(&key);
            }
        }
    }

    /// Sets a header, replacing any earlier casing of the same name.
    #[must_use]
    pub fn with(mut self, name: &str, value: impl Into<Value>) -> Self {
        self.apply(name, value);
        self
    }

    /// The value stored under `name`, compared case-insensitively.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.entries
            .get(&name.to_lowercase())
            .map(|(_, value)| value.as_str())
    }

    /// Whether `name` is present, compared case-insensitively.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(&name.to_lowercase())
    }

    /// Every header as it should go on the wire, in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries
            .values()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }

    /// How many headers are set.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no headers are set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<'a> IntoIterator for &'a Headers {
    type Item = (&'a str, &'a str);
    type IntoIter = Box<dyn Iterator<Item = (&'a str, &'a str)> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.iter())
    }
}

#[cfg(test)]
mod tests {
    use super::{Headers, Value};

    #[test]
    fn a_later_source_wins_regardless_of_casing() {
        let headers = Headers::new()
            .with("Content-Type", "text/plain")
            .with("content-type", "application/json");
        assert_eq!(headers.len(), 1);
        assert_eq!(headers.get("CONTENT-TYPE"), Some("application/json"));
    }

    #[test]
    fn the_last_writer_spelling_is_what_goes_on_the_wire() {
        let headers = Headers::new()
            .with("content-type", "text/plain")
            .with("Content-Type", "application/json");
        let names: Vec<&str> = headers.iter().map(|(name, _)| name).collect();
        assert_eq!(names, vec!["Content-Type"]);
    }

    #[test]
    fn removal_deletes_rather_than_emptying() {
        let headers = Headers::new()
            .with("Content-Type", "application/json")
            .with("content-type", Value::Remove);
        assert!(!headers.contains("Content-Type"));
        assert!(headers.is_empty());
    }

    #[test]
    fn removing_an_absent_header_is_not_an_error() {
        let headers = Headers::new().with("x-absent", Value::Remove);
        assert!(headers.is_empty());
    }

    #[test]
    fn insertion_order_is_preserved() {
        let headers = Headers::new().with("a", "1").with("b", "2").with("c", "3");
        let names: Vec<&str> = headers.iter().map(|(name, _)| name).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }
}
