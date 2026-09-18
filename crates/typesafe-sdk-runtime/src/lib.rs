//! The runtime descriptor sent as `X-TypeSafe-Runtime`.
//!
//! The JavaScript SDK reports which JS runtime is executing — `node/24.1.0
//! (darwin; arm64)`, `bun/…`, `cloudflare-workers`. The Rust equivalent has one
//! answer, so this reports the toolchain that built the binary and the platform
//! it is running on, in the same shape.

/// The toolchain version, captured at build time.
const RUSTC: &str = env!("TYPESAFE_RUSTC_VERSION");

/// The value for the `X-TypeSafe-Runtime` header.
///
/// Computed once and identical for the life of the process, matching the JS
/// SDK, which caches it at module load.
#[must_use]
pub fn describe() -> &'static str {
    static DESCRIPTION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    DESCRIPTION.get_or_init(|| {
        format!(
            "rust/{RUSTC} ({}; {})",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })
}

#[cfg(test)]
mod tests {
    use super::describe;

    /// The JS suite pins the shape of this header, so this one does too.
    #[test]
    fn the_shape_matches_the_documented_format() {
        let value = describe();
        let (name, platform) = value
            .split_once(" (")
            .unwrap_or_else(|| panic!("no platform section in {value:?}"));

        let (runtime, version) = name
            .split_once('/')
            .unwrap_or_else(|| panic!("no version in {name:?}"));
        assert_eq!(runtime, "rust");
        assert_eq!(
            version.split('.').count(),
            3,
            "expected a three-part version, got {version:?}"
        );
        assert!(
            version
                .split('.')
                .all(|part| part.chars().all(|c| c.is_ascii_digit())),
            "non-numeric version {version:?}"
        );

        let platform = platform
            .strip_suffix(')')
            .unwrap_or_else(|| panic!("unterminated platform in {value:?}"));
        let (os, arch) = platform
            .split_once("; ")
            .unwrap_or_else(|| panic!("no arch in {platform:?}"));
        assert!(!os.is_empty() && !arch.is_empty());
    }

    #[test]
    fn it_is_stable_across_calls() {
        assert_eq!(describe(), describe());
    }
}
