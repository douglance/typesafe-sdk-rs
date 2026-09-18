//! Derive macro turning a struct into typed questions.
//!
//! TypeScript infers answer types from the question literals, so a caller
//! writes `answers.category.choice` and gets a union of exactly the labels they
//! asked about. Rust cannot infer that from a runtime map, so this macro does
//! it at compile time instead — and arrives somewhere stronger, because the
//! generated label enum makes a `match` over the outcomes exhaustive.
//!
//! ```ignore
//! #[derive(Questions)]
//! struct Ticket {
//!     /// What is this ticket about?
//!     #[choice(billing, technical, other)]
//!     category: Choice,
//!     /// How urgent is it?
//!     #[score("can wait", "this week", "today")]
//!     urgency: Score,
//! }
//! ```
//!
//! generates `Ticket::questions()`, an enum `TicketCategory` with one variant
//! per label, and `Ticket::answers(&response)` returning a struct whose
//! `category` field is that enum rather than a `String`.

mod emit;
mod labels;
mod parse;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Derives a typed question set from a struct.
///
/// # Panics
/// Does not panic; malformed input becomes a compile error naming the field.
#[proc_macro_derive(Questions, attributes(choice, score, noul))]
pub fn questions(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match parse::model(&input).map(|model| emit::generate(&model)) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
