//! Reading the struct the caller wrote.

use syn::{Attribute, Data, DeriveInput, Error, Expr, Fields, Ident, Lit, Result};

/// One question, as declared on a field.
pub(crate) enum Kind {
    /// A yes/no question.
    Noul,
    /// A selection between labels.
    Choice(Vec<String>),
    /// A score against an ordered rubric.
    Score(Vec<String>),
}

/// One field of the question struct.
pub(crate) struct Question {
    /// The field's name, which becomes the answer's name.
    pub(crate) name: Ident,
    /// The question text, taken from the doc comment.
    pub(crate) instructions: String,
    /// What kind of question it is.
    pub(crate) kind: Kind,
}

/// A whole question struct.
pub(crate) struct Model {
    /// The struct's name.
    pub(crate) name: Ident,
    /// Its questions, in declaration order.
    pub(crate) questions: Vec<Question>,
}

/// Reads the derive input into a model.
///
/// # Errors
/// Returns a compile error when the input is not a struct with named fields,
/// or a field does not carry exactly one question attribute.
pub(crate) fn model(input: &DeriveInput) -> Result<Model> {
    let fields = named_fields(input)?;
    Ok(Model {
        name: input.ident.clone(),
        questions: fields.named.iter().map(question).collect::<Result<_>>()?,
    })
}

/// The named fields of the struct being derived.
fn named_fields(input: &DeriveInput) -> Result<&syn::FieldsNamed> {
    let Data::Struct(data) = &input.data else {
        return Err(Error::new_spanned(
            input,
            "Questions can only be derived for a struct",
        ));
    };
    match &data.fields {
        Fields::Named(fields) => Ok(fields),
        _ => Err(Error::new_spanned(
            input,
            "Questions needs named fields; each one becomes an answer",
        )),
    }
}

/// Reads one field into a question.
fn question(field: &syn::Field) -> Result<Question> {
    let name = field
        .ident
        .clone()
        .ok_or_else(|| Error::new_spanned(field, "every question field must be named"))?;
    Ok(Question {
        instructions: doc_of(&field.attrs),
        kind: kind_of(&field.attrs, &name)?,
        name,
    })
}

/// The doc comment, joined, which becomes the question text.
fn doc_of(attrs: &[Attribute]) -> String {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| match &attr.meta {
            syn::Meta::NameValue(nv) => literal_string(&nv.value),
            _ => None,
        })
        .map(|line| line.trim().to_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

fn literal_string(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Str(text) => Some(text.value()),
            _ => None,
        },
        _ => None,
    }
}

/// Which question attribute the field carries.
fn kind_of(attrs: &[Attribute], name: &Ident) -> Result<Kind> {
    let found: Vec<&Attribute> = attrs
        .iter()
        .filter(|attr| {
            ["noul", "choice", "score"]
                .iter()
                .any(|kind| attr.path().is_ident(kind))
        })
        .collect();

    match found.as_slice() {
        [] => Err(Error::new_spanned(
            name,
            "each field needs one of #[noul], #[choice(..)] or #[score(..)]",
        )),
        [attr] => one(attr),
        _ => Err(Error::new_spanned(
            name,
            "a field may carry only one question attribute",
        )),
    }
}

fn one(attr: &Attribute) -> Result<Kind> {
    if attr.path().is_ident("noul") {
        return Ok(Kind::Noul);
    }
    let entries = entries_of(attr)?;
    if attr.path().is_ident("choice") {
        return Ok(Kind::Choice(entries));
    }
    if entries.len() < 2 {
        return Err(Error::new_spanned(
            attr,
            "a score rubric needs at least two entries; one score cannot express a comparison",
        ));
    }
    Ok(Kind::Score(entries))
}

/// The labels or rubric entries inside `#[choice(..)]` or `#[score(..)]`.
///
/// Both bare identifiers and string literals are accepted, so a simple label
/// list reads as `#[choice(billing, technical)]` and one needing spaces or
/// punctuation reads as `#[score("can wait", "this week")]`.
fn entries_of(attr: &Attribute) -> Result<Vec<String>> {
    let mut entries = Vec::new();
    attr.parse_nested_meta(|meta| {
        if let Some(ident) = meta.path.get_ident() {
            entries.push(ident.to_string());
            return Ok(());
        }
        Err(meta.error("expected a label or a string"))
    })
    .or_else(|_| strings_of(attr, &mut entries))?;

    if entries.is_empty() {
        return Err(Error::new_spanned(attr, "expected at least one entry"));
    }
    Ok(entries)
}

/// Falls back to parsing a comma-separated list of string literals.
fn strings_of(attr: &Attribute, entries: &mut Vec<String>) -> Result<()> {
    entries.clear();
    let parsed = attr.parse_args_with(
        syn::punctuated::Punctuated::<syn::LitStr, syn::Token![,]>::parse_terminated,
    )?;
    entries.extend(parsed.iter().map(syn::LitStr::value));
    Ok(())
}
