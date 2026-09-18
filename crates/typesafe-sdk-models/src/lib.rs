//! Model cards and the models listing.
//!
//! Unwrapping is fallible on purpose. A malformed response and an account with
//! no models would otherwise both arrive as an empty list, and only one of
//! those is worth waking someone for.

use serde::{Deserialize, Serialize};
use typesafe_sdk_error::{Error, Result};

/// One model available to the account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCard {
    /// The name to pass as `model`, such as `jev-latest`.
    pub name: String,
    /// What the model is, in prose.
    pub description: String,
    /// When it was released, as an ISO 8601 timestamp.
    pub release_date: String,
}

/// The envelope `GET /v1/models` answers with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Listing {
    models: Vec<ModelCard>,
}

/// Unwraps a models listing into the cards it contains.
///
/// # Errors
/// Returns [`Error::Invalid`] when the body is not the documented envelope,
/// rather than silently yielding an empty list — an empty account and a
/// malformed response should not look the same to a caller.
pub fn unwrap_listing(body: &serde_json::Value) -> Result<Vec<ModelCard>> {
    serde_json::from_value::<Listing>(body.clone())
        .map(|listing| listing.models)
        .map_err(|_| {
            Error::Invalid(
                "Unexpected response shape from GET /v1/models; expected { models: [...] }."
                    .to_owned(),
            )
        })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a test that cannot fail loudly is not a test"
)]
mod tests {
    use super::unwrap_listing;
    use serde_json::json;

    /// Captured verbatim from `GET /v1/models`.
    fn live_body() -> serde_json::Value {
        json!({"models": [
            {"name": "jev-latest",
             "description": "The latest iteration of TypeSafe's System One Model: Jev",
             "release_date": "2026-09-10T18:38:01.391457+00:00"},
            {"name": "jev-preview",
             "description": "A preview version of `jev-latest`: should be better in most ways",
             "release_date": "2026-09-10T18:39:06.057655+00:00"}
        ]})
    }

    #[test]
    fn the_live_listing_unwraps_to_its_cards() {
        let cards = unwrap_listing(&live_body()).unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].name, "jev-latest");
        assert_eq!(cards[1].name, "jev-preview");
        assert!(cards[0].release_date.starts_with("2026-09-10"));
    }

    #[test]
    fn an_empty_listing_is_valid() {
        assert_eq!(unwrap_listing(&json!({"models": []})).unwrap(), vec![]);
    }

    #[test]
    fn a_wrong_shape_is_an_error_not_an_empty_list() {
        for body in [json!({}), json!({"models": 7}), json!([]), json!(null)] {
            let message = unwrap_listing(&body).unwrap_err().to_string();
            assert_eq!(
                message,
                "Unexpected response shape from GET /v1/models; expected { models: [...] }."
            );
        }
    }

    /// Fields the SDK does not model must not break parsing.
    #[test]
    fn unknown_fields_on_a_card_are_tolerated() {
        let body = json!({"models": [{
            "name": "jev-latest", "description": "d", "release_date": "r",
            "tags": ["internal"]
        }]});
        assert_eq!(unwrap_listing(&body).unwrap()[0].name, "jev-latest");
    }
}
