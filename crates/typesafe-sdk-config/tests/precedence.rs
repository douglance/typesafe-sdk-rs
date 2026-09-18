//! Where each setting comes from, and what is refused.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot fail loudly is not a test"
)]

use typesafe_sdk_config::{Builder, Config, DEFAULT_BASE_URL, DEFAULT_MODEL};
use typesafe_sdk_env::Fixed;
use typesafe_sdk_log::Level;

fn built(env: &[(&str, &str)]) -> Config {
    Builder::new().build(&Fixed::new(env)).unwrap()
}

#[test]
fn the_defaults_apply_when_only_a_key_is_present() {
    let config = built(&[("TYPESAFE_API_KEY", "sk_live_x")]);
    assert_eq!(config.base_url, DEFAULT_BASE_URL);
    assert_eq!(config.default_model, DEFAULT_MODEL);
    assert_eq!(config.timeout_ms, 10_000);
    assert_eq!(config.log_level(), Level::Warn);
}

#[test]
fn every_setting_can_come_from_the_environment() {
    let config = built(&[
        ("TYPESAFE_API_KEY", "sk_live_x"),
        ("TYPESAFE_BASE_URL", "https://env.test"),
        ("TYPESAFE_DEFAULT_MODEL", "jev-preview"),
        ("TYPESAFE_LOG_LEVEL", "debug"),
    ]);
    assert_eq!(config.base_url, "https://env.test");
    assert_eq!(config.default_model, "jev-preview");
    assert_eq!(config.log_level(), Level::Debug);
}

#[test]
fn an_explicit_setting_beats_the_environment() {
    let env = Fixed::new(&[
        ("TYPESAFE_API_KEY", "from_env"),
        ("TYPESAFE_BASE_URL", "https://env.test"),
    ]);
    let config = Builder::new()
        .api_key("from_code")
        .base_url("https://code.test")
        .build(&env)
        .unwrap();
    assert_eq!(config.base_url, "https://code.test");
    assert_eq!(config.authorization(), "Bearer from_code");
}

#[test]
fn trailing_slashes_are_stripped_from_either_source() {
    let from_env = built(&[
        ("TYPESAFE_API_KEY", "k"),
        ("TYPESAFE_BASE_URL", "https://env.test///"),
    ]);
    assert_eq!(from_env.base_url, "https://env.test");

    let from_code = Builder::new()
        .api_key("k")
        .base_url("https://code.test//")
        .build(&Fixed::default())
        .unwrap();
    assert_eq!(from_code.base_url, "https://code.test");
}

#[test]
fn a_missing_key_names_the_environment_variable() {
    let message = Builder::new()
        .build(&Fixed::default())
        .unwrap_err()
        .to_string();
    assert!(message.contains("TYPESAFE_API_KEY"), "{message}");
    assert!(message.starts_with("No API key was provided."), "{message}");
}

/// A shell profile exporting an empty key should report "no key", not try to
/// authenticate with an empty string.
#[test]
fn a_blank_key_in_the_environment_counts_as_missing() {
    let message = Builder::new()
        .build(&Fixed::new(&[("TYPESAFE_API_KEY", "   ")]))
        .unwrap_err()
        .to_string();
    assert!(message.starts_with("No API key was provided."), "{message}");
}

#[test]
fn an_invalid_log_level_names_its_source() {
    let message = Builder::new()
        .api_key("k")
        .build(&Fixed::new(&[("TYPESAFE_LOG_LEVEL", "chatty")]))
        .unwrap_err()
        .to_string();
    assert_eq!(
        message,
        "Invalid log level \"chatty\" from TYPESAFE_LOG_LEVEL. \
         Expected one of: debug, info, warn, error, off."
    );
}

#[test]
fn a_zero_timeout_is_refused() {
    let message = Builder::new()
        .api_key("k")
        .timeout_ms(0)
        .build(&Fixed::default())
        .unwrap_err()
        .to_string();
    assert_eq!(
        message,
        "`timeout` must be a positive number of milliseconds, got 0."
    );
}

/// The key must not be reachable through `Debug`, which is how credentials end
/// up in logs and issue reports.
#[test]
fn the_key_never_appears_in_debug_output() {
    let config = Builder::new()
        .api_key("sk_live_0123456789abcdef")
        .build(&Fixed::default())
        .unwrap();
    let rendered = format!("{config:?}");
    assert!(!rendered.contains("sk_live"), "{rendered}");
    assert!(!rendered.contains("0123456789"), "{rendered}");
    assert!(rendered.contains("***cdef"), "{rendered}");
    assert_eq!(config.key_hint(), "***cdef");
}

#[test]
fn a_short_key_reveals_nothing_in_its_hint() {
    let config = Builder::new()
        .api_key("short")
        .build(&Fixed::default())
        .unwrap();
    assert_eq!(config.key_hint(), "***");
}

#[test]
fn an_impossible_retry_policy_is_refused_at_construction() {
    let policy = typesafe_sdk_retry::RetryPolicy {
        backoff_jitter: 2.0,
        ..typesafe_sdk_retry::RetryPolicy::default()
    };
    let message = Builder::new()
        .api_key("k")
        .retry(policy)
        .build(&Fixed::default())
        .unwrap_err()
        .to_string();
    assert!(message.contains("retry.backoffJitter"), "{message}");
}
