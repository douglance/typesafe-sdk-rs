//! Writing the typed question set, label enums and answers struct.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::labels::enum_name;

use crate::labels;
use crate::parse::{Kind, Model, Question};

/// Generates everything the derive produces.
pub(crate) fn generate(model: &Model) -> TokenStream {
    let enums = model
        .questions
        .iter()
        .filter_map(|question| labels::label_enum(model, question));
    let answers = answers_struct(model);
    let questions = questions_fn(model);
    let marker = marker(model);
    quote! {
        #(#enums)*
        #answers
        #questions
        #marker
    }
}

/// Marks the declaration's fields as read.
///
/// The fields of a question struct carry no data — their names *are* the
/// question names — so every caller would otherwise get a dead-code warning for
/// writing the declaration correctly. An anonymous const is always live, so the
/// reads count and nothing has to suppress a lint to make the wart go away.
fn marker(model: &Model) -> TokenStream {
    let name = &model.name;
    let fields = model.questions.iter().map(|question| &question.name);
    quote! {
        const _: fn(&#name) = |declaration| {
            #( let _ = &declaration.#fields; )*
        };
    }
}

/// The answers struct name, such as `TicketAnswers`.
fn answers_name(model: &Model) -> Ident {
    format_ident!("{}Answers", model.name)
}

/// The answers struct, with one typed field per question.
fn answers_struct(model: &Model) -> TokenStream {
    let name = answers_name(model);
    let doc = format!("Typed answers to [`{}`].", model.name);
    let fields = model.questions.iter().map(|question| {
        let field = &question.name;
        let ty = answer_type(model, question);
        let field_doc = format!("The answer to `{field}`.");
        quote! { #[doc = #field_doc] pub #field: #ty, }
    });

    quote! {
        #[doc = #doc]
        #[derive(Debug, Clone)]
        pub struct #name { #(#fields)* }
    }
}

/// The Rust type one answer takes.
fn answer_type(model: &Model, question: &Question) -> TokenStream {
    match question.kind {
        Kind::Noul | Kind::Score(_) => quote! { f64 },
        Kind::Choice(_) => {
            let name = enum_name(model, question);
            quote! { #name }
        }
    }
}

/// `questions()` and `answers()` on the struct itself.
fn questions_fn(model: &Model) -> TokenStream {
    let name = &model.name;
    let answers = answers_name(model);
    let builders = model.questions.iter().map(builder_for);
    let extractors = model
        .questions
        .iter()
        .map(|question| extractor_for(model, question));

    quote! {
        impl #name {
            /// The question set, ready to send.
            #[must_use]
            pub fn questions() -> ::typesafe_sdk_questions::Questions {
                let mut set = ::typesafe_sdk_questions::Questions::new();
                #(#builders)*
                set
            }

            /// Reads the typed answers out of a response.
            ///
            /// # Errors
            /// Returns an error when an answer is missing or has a different
            /// shape from the question that was asked.
            pub fn answers(
                response: &::typesafe_sdk_answers::SystemOneResponse,
            ) -> ::core::result::Result<#answers, ::typesafe_sdk_error::Error> {
                Ok(#answers { #(#extractors)* })
            }
        }
    }
}

/// One line of `questions()`.
fn builder_for(question: &Question) -> TokenStream {
    let key = question.name.to_string();
    let text = &question.instructions;
    let built = build_call(&question.kind, text);
    quote! { set.insert(#key.to_owned(), #built); }
}

/// The builder call for one question kind.
fn build_call(kind: &Kind, text: &str) -> TokenStream {
    match kind {
        Kind::Noul => noul_call(text),
        Kind::Choice(labels) => choice_call(text, labels),
        Kind::Score(entries) => score_call(text, entries),
    }
}

fn noul_call(text: &str) -> TokenStream {
    quote! { ::typesafe_sdk_questions::noul(#text) }
}

fn choice_call(text: &str, labels: &[String]) -> TokenStream {
    quote! { ::typesafe_sdk_questions::choice_of(#text, [ #(#labels),* ]) }
}

fn score_call(text: &str, entries: &[String]) -> TokenStream {
    quote! { ::typesafe_sdk_questions::score(#text, [ #(#entries),* ]) }
}

/// One field of the answers struct, read from the response.
fn extractor_for(model: &Model, question: &Question) -> TokenStream {
    let field = &question.name;
    let key = field.to_string();
    let wrong = format!("answer \"{key}\" came back in a different shape from the question asked");

    let value = match question.kind {
        Kind::Noul => quote! { answer.as_noul().map(|a| a.noul) },
        Kind::Score(_) => quote! { answer.as_score().map(|a| a.score) },
        Kind::Choice(_) => {
            let name = enum_name(model, question);
            quote! {
                answer
                    .as_choice()
                    .and_then(|a| #name::from_label(&a.choice))
            }
        }
    };

    quote! {
        #field: {
            let answer = response.expect(#key)?;
            #value.ok_or_else(|| {
                ::typesafe_sdk_error::Error::Invalid(#wrong.to_owned())
            })?
        },
    }
}
