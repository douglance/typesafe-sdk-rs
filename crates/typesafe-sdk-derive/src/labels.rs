//! The enum generated for each choice question.
//!
//! This is what makes a `match` over the outcomes exhaustive, and a mistyped
//! label a compile error rather than a comparison that silently never matches.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::Ident;

use crate::parse::{Kind, Model, Question};

/// The enum name for a choice field, such as `TicketCategory`.
pub(crate) fn enum_name(model: &Model, question: &Question) -> Ident {
    format_ident!("{}{}", model.name, pascal(&question.name.to_string()))
}

/// One enum per choice question; nothing for the other kinds.
pub(crate) fn label_enum(model: &Model, question: &Question) -> Option<TokenStream> {
    let Kind::Choice(labels) = &question.kind else {
        return None;
    };
    Some(define(
        &enum_name(model, question),
        labels,
        &question.name.to_string(),
    ))
}

/// The enum, its labels, and the conversions between them.
fn define(name: &Ident, labels: &[String], field: &str) -> TokenStream {
    let variants: Vec<Ident> = labels
        .iter()
        .map(|label| ident_of(&pascal(label)))
        .collect();
    let declaration = declare(name, labels, &variants, field);
    let conversions = convert(name, labels, &variants);
    quote! { #declaration #conversions }
}

/// The enum declaration itself.
fn declare(name: &Ident, texts: &[String], variants: &[Ident], field: &str) -> TokenStream {
    let doc = format!("The labels `{field}` may be answered with.");
    quote! {
        #[doc = #doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum #name {
            #( #[doc = #texts] #variants, )*
        }
    }
}

/// Conversions between a variant and the label on the wire.
fn convert(name: &Ident, texts: &[String], variants: &[Ident]) -> TokenStream {
    quote! {
        impl #name {
            /// The label as it goes on the wire.
            #[must_use]
            pub const fn label(self) -> &'static str {
                match self { #( Self::#variants => #texts, )* }
            }

            /// The variant for a label the service returned.
            #[must_use]
            pub fn from_label(label: &str) -> Option<Self> {
                match label { #( #texts => Some(Self::#variants), )* _ => None }
            }
        }

        impl ::core::fmt::Display for #name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(self.label())
            }
        }
    }
}

/// Turns a `snake_case` name into `PascalCase` for an identifier.
fn pascal(text: &str) -> String {
    text.split(['_', '-', ' '])
        .filter(|part| !part.is_empty())
        .map(capitalise)
        .collect()
}

/// Upper-cases the first character and lower-cases the rest.
///
fn capitalise(part: &str) -> String {
    let mut chars = part.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
    })
}

/// A syntactically valid identifier, or a placeholder that fails to compile
/// with the offending text visible in the error.
fn ident_of(text: &str) -> Ident {
    syn::parse_str::<Ident>(text)
        .unwrap_or_else(|_| Ident::new(&format!("Invalid_{}", text.len()), Span::call_site()))
}
