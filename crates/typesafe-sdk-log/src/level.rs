//! How much the SDK says.
//!
//! Nothing here is ever emitted above `info`. A failed request is returned to
//! the caller, and logging it as well would report someone else's decision
//! twice — which is why the default level of `warn` leaves a healthy client
//! completely silent.

use typesafe_sdk_error::{Error, Result};

/// Every level, from most to least verbose.
pub const LEVELS: [Level; 5] = [
    Level::Debug,
    Level::Info,
    Level::Warn,
    Level::Error,
    Level::Off,
];

/// The level a client uses when none is configured.
pub const DEFAULT_LEVEL: Level = Level::Warn;

/// How much detail reaches the sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Request and response bodies, and the headers, redacted.
    Debug,
    /// One line per request and per retry.
    Info,
    /// Reserved; the SDK emits nothing at this level.
    Warn,
    /// Reserved; the SDK emits nothing at this level.
    Error,
    /// Nothing at all.
    Off,
}

impl Level {
    /// The level's name as it is written in configuration.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
            Self::Off => "off",
        }
    }

    /// Whether a message at `self` is emitted by a logger set to `threshold`.
    #[must_use]
    pub fn passes(self, threshold: Self) -> bool {
        self >= threshold && self != Self::Off
    }

    /// Parses a level, naming where the bad value came from.
    ///
    /// # Errors
    /// Returns [`Error::Invalid`] listing the levels that would have worked.
    pub fn parse(value: &str, source: &str) -> Result<Self> {
        LEVELS
            .into_iter()
            .find(|level| level.name() == value)
            .ok_or_else(|| {
                let valid = LEVELS
                    .iter()
                    .map(|level| level.name())
                    .collect::<Vec<_>>()
                    .join(", ");
                Error::Invalid(format!(
                    "Invalid log level \"{value}\" from {source}. Expected one of: {valid}."
                ))
            })
    }
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a test that cannot fail loudly is not a test"
)]
mod tests {
    use super::{DEFAULT_LEVEL, LEVELS, Level};

    #[test]
    fn the_levels_are_ordered_from_most_to_least_verbose() {
        let names: Vec<&str> = LEVELS.iter().map(|l| l.name()).collect();
        assert_eq!(names, vec!["debug", "info", "warn", "error", "off"]);
    }

    #[test]
    fn the_default_silences_a_healthy_client() {
        assert_eq!(DEFAULT_LEVEL, Level::Warn);
        assert!(!Level::Debug.passes(DEFAULT_LEVEL));
        assert!(!Level::Info.passes(DEFAULT_LEVEL));
    }

    #[test]
    fn a_threshold_admits_itself_and_anything_louder() {
        assert!(Level::Info.passes(Level::Debug));
        assert!(Level::Info.passes(Level::Info));
        assert!(!Level::Debug.passes(Level::Info));
    }

    #[test]
    fn off_admits_nothing_at_all() {
        for level in LEVELS {
            assert!(!level.passes(Level::Off), "{level} escaped `off`");
        }
    }

    #[test]
    fn an_unknown_level_names_its_source_and_the_valid_ones() {
        let message = Level::parse("chatty", "TYPESAFE_LOG_LEVEL")
            .unwrap_err()
            .to_string();
        assert_eq!(
            message,
            "Invalid log level \"chatty\" from TYPESAFE_LOG_LEVEL. \
             Expected one of: debug, info, warn, error, off."
        );
    }

    #[test]
    fn every_level_round_trips_through_its_name() {
        for level in LEVELS {
            assert_eq!(Level::parse(level.name(), "test").unwrap(), level);
        }
    }
}
