//! Environment variable names and precedence.
//!
//! One rule, applied everywhere: an explicit argument wins, then the
//! environment, then the SDK default. A variable set to whitespace counts as
//! unset — an empty `TYPESAFE_API_KEY` exported by a shell profile should
//! produce "no key was provided", not an attempt to authenticate with "".

/// The environment variable a setting may be read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Var {
    /// The API key.
    ApiKey,
    /// The API root.
    BaseUrl,
    /// The model used when a request does not name one.
    DefaultModel,
    /// The log level.
    LogLevel,
}

impl Var {
    /// The variable's name as exported in a shell.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ApiKey => "TYPESAFE_API_KEY",
            Self::BaseUrl => "TYPESAFE_BASE_URL",
            Self::DefaultModel => "TYPESAFE_DEFAULT_MODEL",
            Self::LogLevel => "TYPESAFE_LOG_LEVEL",
        }
    }

    /// Every variable the SDK reads.
    #[must_use]
    pub const fn all() -> [Self; 4] {
        [
            Self::ApiKey,
            Self::BaseUrl,
            Self::DefaultModel,
            Self::LogLevel,
        ]
    }
}

impl std::fmt::Display for Var {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Somewhere to read environment variables from.
///
/// A trait rather than direct `std::env` access so tests can supply a fixed
/// environment; process environment mutation is not thread-safe.
pub trait Source {
    /// The raw value of `name`, before trimming.
    fn get(&self, name: &str) -> Option<String>;
}

/// The real process environment.
#[derive(Debug, Clone, Copy, Default)]
pub struct Process;

impl Source for Process {
    fn get(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
}

/// A fixed environment, for tests.
#[derive(Debug, Clone, Default)]
pub struct Fixed(Vec<(String, String)>);

impl Fixed {
    /// Builds an environment from name/value pairs.
    #[must_use]
    pub fn new(pairs: &[(&str, &str)]) -> Self {
        Self(
            pairs
                .iter()
                .map(|&(name, value)| (name.to_owned(), value.to_owned()))
                .collect(),
        )
    }
}

impl Source for Fixed {
    fn get(&self, name: &str) -> Option<String> {
        self.0
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    }
}

/// Reads `var`, treating blank and whitespace-only values as unset.
#[must_use]
pub fn read(source: &impl Source, var: Var) -> Option<String> {
    let raw = source.get(var.name())?;
    let trimmed = raw.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// The explicit value if given, otherwise the environment.
#[must_use]
pub fn from_code_or_env(
    from_code: Option<String>,
    source: &impl Source,
    var: Var,
) -> Option<String> {
    from_code.or_else(|| read(source, var))
}

#[cfg(test)]
mod tests {
    use super::{Fixed, Var, from_code_or_env, read};

    #[test]
    fn the_names_are_the_documented_ones() {
        assert_eq!(Var::ApiKey.name(), "TYPESAFE_API_KEY");
        assert_eq!(Var::BaseUrl.name(), "TYPESAFE_BASE_URL");
        assert_eq!(Var::DefaultModel.name(), "TYPESAFE_DEFAULT_MODEL");
        assert_eq!(Var::LogLevel.name(), "TYPESAFE_LOG_LEVEL");
    }

    #[test]
    fn a_value_is_trimmed() {
        let env = Fixed::new(&[("TYPESAFE_API_KEY", "  sk_live  ")]);
        assert_eq!(read(&env, Var::ApiKey).as_deref(), Some("sk_live"));
    }

    #[test]
    fn a_blank_value_counts_as_unset() {
        for blank in ["", " ", "\t\n"] {
            let env = Fixed::new(&[("TYPESAFE_API_KEY", blank)]);
            assert_eq!(read(&env, Var::ApiKey), None, "{blank:?} should be unset");
        }
    }

    #[test]
    fn an_explicit_value_beats_the_environment() {
        let env = Fixed::new(&[("TYPESAFE_BASE_URL", "https://env.test")]);
        let chosen = from_code_or_env(Some("https://code.test".to_owned()), &env, Var::BaseUrl);
        assert_eq!(chosen.as_deref(), Some("https://code.test"));
    }

    #[test]
    fn the_environment_is_the_fallback() {
        let env = Fixed::new(&[("TYPESAFE_BASE_URL", "https://env.test")]);
        assert_eq!(
            from_code_or_env(None, &env, Var::BaseUrl).as_deref(),
            Some("https://env.test")
        );
    }
}
