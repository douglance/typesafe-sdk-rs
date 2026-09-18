//! What came back in a response body.
//!
//! The content type is deliberately ignored. Services mislabel error pages, and
//! a body that parses as JSON is more useful parsed whatever it claims to be —
//! so the only distinction kept is between absent, parsed, and quoted verbatim.

use serde_json::Value;

/// A response body as the client managed to read it.
///
/// The distinction matters for messages: an absent body reads as "no body",
/// while an unparseable one is quoted back so the reader can see what arrived.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Body {
    /// No body at all, or an empty one.
    #[default]
    Empty,
    /// A body that parsed as JSON.
    Json(Value),
    /// A body that did not parse as JSON, kept verbatim.
    Text(String),
}

impl Body {
    /// Parses `raw`, falling back to text, exactly as the JS SDK does.
    ///
    /// The content type is deliberately ignored: servers mislabel error pages,
    /// and a body that parses as JSON is more useful parsed whatever it claims.
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        if raw.is_empty() {
            return Self::Empty;
        }
        serde_json::from_str(raw).map_or_else(|_| Self::Text(raw.to_owned()), Self::Json)
    }

    /// The JSON value, when the body parsed as JSON.
    #[must_use]
    pub const fn json(&self) -> Option<&Value> {
        match self {
            Self::Json(value) => Some(value),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Body;

    #[test]
    fn an_empty_body_is_empty() {
        assert_eq!(Body::parse(""), Body::Empty);
    }

    #[test]
    fn json_is_parsed_into_the_value_it_describes() {
        assert_eq!(
            Body::parse(r#"{"a":1}"#).json(),
            Some(&serde_json::json!({"a": 1}))
        );
    }

    #[test]
    fn an_html_error_page_is_kept_verbatim() {
        assert_eq!(
            Body::parse("<h1>bad gateway</h1>"),
            Body::Text("<h1>bad gateway</h1>".to_owned())
        );
    }
}
