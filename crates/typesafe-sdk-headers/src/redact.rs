//! Masking credentials before they reach a log.
//!
//! The rule is deliberately conservative: keep the scheme so a reader can tell
//! `Bearer` from `Basic`, and keep the last four characters only when the
//! secret is long enough that four characters cannot narrow it usefully.

/// Headers whose value is a credential with an optional scheme prefix.
const KEY_HEADERS: &[&str] = &["authorization", "proxy-authorization", "x-api-key"];

/// Headers whose value is replaced in full.
const OPAQUE_HEADERS: &[&str] = &["cookie", "set-cookie"];

/// Below this length, no tail is revealed at all.
const MIN_SECRET_FOR_TAIL: usize = 8;

/// Characters of the secret kept as a recognisable tail.
const TAIL: usize = 4;

/// Masks `value` according to what `name` is known to carry.
#[must_use]
pub fn redact(name: &str, value: &str) -> String {
    let lower = name.to_lowercase();
    if KEY_HEADERS.contains(&lower.as_str()) {
        return redact_key(value);
    }
    if OPAQUE_HEADERS.contains(&lower.as_str()) {
        return "***".to_owned();
    }
    value.to_owned()
}

/// Keeps the scheme and, for long secrets only, the last four characters.
fn redact_key(value: &str) -> String {
    let (scheme, secret) = split_scheme(value);
    let tail = if secret.chars().count() > MIN_SECRET_FOR_TAIL {
        let skip = secret.chars().count() - TAIL;
        secret.chars().skip(skip).collect()
    } else {
        String::new()
    };
    scheme.map_or_else(
        || format!("***{tail}"),
        |scheme| format!("{scheme} ***{tail}"),
    )
}

/// Splits `Bearer sk_live_…` into its scheme and secret; a bare secret has none.
fn split_scheme(value: &str) -> (Option<&str>, &str) {
    match value.split_once(char::is_whitespace) {
        Some((scheme, rest)) => (Some(scheme), rest.trim_start()),
        None => (None, value),
    }
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn a_long_bearer_token_keeps_its_scheme_and_last_four() {
        assert_eq!(
            redact("Authorization", "Bearer sk_live_0123456789abcdef"),
            "Bearer ***cdef"
        );
    }

    #[test]
    fn a_short_secret_reveals_nothing() {
        assert_eq!(redact("authorization", "Bearer abc"), "Bearer ***");
    }

    #[test]
    fn a_bare_key_has_no_scheme() {
        assert_eq!(redact("x-api-key", "0123456789abcdef"), "***cdef");
    }

    #[test]
    fn cookies_are_replaced_in_full() {
        assert_eq!(redact("Set-Cookie", "session=abcdef123456"), "***");
    }

    #[test]
    fn an_ordinary_header_passes_through() {
        assert_eq!(redact("X-Team", "platform"), "platform");
    }

    #[test]
    fn matching_ignores_case() {
        assert_eq!(redact("AUTHORIZATION", "Bearer abc"), "Bearer ***");
    }

    /// Exactly eight characters is not "longer than eight".
    #[test]
    fn the_tail_boundary_is_exclusive() {
        assert_eq!(redact("x-api-key", "01234567"), "***");
        assert_eq!(redact("x-api-key", "012345678"), "***5678");
    }
}
