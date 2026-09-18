//! Reading a server's requested delay out of response headers.
//!
//! This lives beside the header map rather than with the retry policy because
//! two unrelated callers need it — the policy, to decide how long to wait, and
//! `RateLimitError`, to report what the server asked for — and the policy sits
//! a layer above the error type. Parsing headers is a header concern.

use crate::merge::Headers;

/// Milliseconds the server asked the caller to wait, if it asked at all.
///
/// `retry-after-ms` wins over `Retry-After` when both are present. `Retry-After`
/// is either a number of seconds or an HTTP date; a date already in the past
/// yields zero rather than a negative wait. `now_ms` is Unix epoch milliseconds,
/// passed in so the date branch is testable.
#[must_use]
pub fn parse_retry_after(headers: &Headers, now_ms: i64) -> Option<u64> {
    from_milliseconds(headers).or_else(|| from_retry_after(headers, now_ms))
}

/// The non-standard `retry-after-ms` header, which takes precedence.
fn from_milliseconds(headers: &Headers) -> Option<u64> {
    let raw = headers.get("retry-after-ms")?;
    let value: f64 = raw.trim().parse().ok()?;
    (value.is_finite() && value >= 0.0).then(|| round_to_u64(value))
}

/// `Retry-After`, as either seconds or an HTTP date.
fn from_retry_after(headers: &Headers, now_ms: i64) -> Option<u64> {
    let raw = headers.get("retry-after")?.trim();
    if let Ok(seconds) = raw.parse::<f64>() {
        if !seconds.is_finite() || seconds < 0.0 {
            return None;
        }
        return Some(round_to_u64(seconds * 1000.0));
    }
    let at = httpdate_ms(raw)?;
    Some(u64::try_from((at - now_ms).max(0)).unwrap_or(0))
}

/// Milliseconds since the Unix epoch for an HTTP date, or `None` if unparseable.
fn httpdate_ms(raw: &str) -> Option<i64> {
    let parsed = httpdate::parse_http_date(raw).ok()?;
    let since = parsed.duration_since(std::time::UNIX_EPOCH).ok()?;
    i64::try_from(since.as_millis()).ok()
}

/// JavaScript's `Number` is a double; matching it keeps fractional seconds exact.
fn round_to_u64(value: f64) -> u64 {
    if value <= 0.0 {
        return 0;
    }
    let rounded = value.round();
    if rounded >= u64::MAX as f64 {
        u64::MAX
    } else {
        rounded as u64
    }
}

#[cfg(test)]
mod tests {
    use super::parse_retry_after;
    use crate::merge::Headers;

    /// 2026-09-18T00:00:00Z, so the date cases have a fixed frame.
    const NOW_MS: i64 = 1_789_689_600_000;

    fn headers(pairs: &[(&str, &str)]) -> Headers {
        pairs
            .iter()
            .fold(Headers::new(), |acc, &(name, value)| acc.with(name, value))
    }

    #[test]
    fn seconds_become_milliseconds() {
        assert_eq!(
            parse_retry_after(&headers(&[("retry-after", "7")]), NOW_MS),
            Some(7_000)
        );
    }

    #[test]
    fn zero_seconds_is_a_delay_of_zero_not_absent() {
        assert_eq!(
            parse_retry_after(&headers(&[("retry-after", "0")]), NOW_MS),
            Some(0)
        );
    }

    #[test]
    fn fractional_seconds_are_kept() {
        assert_eq!(
            parse_retry_after(&headers(&[("retry-after", "1.5")]), NOW_MS),
            Some(1_500)
        );
    }

    #[test]
    fn milliseconds_win_over_seconds() {
        let h = headers(&[("retry-after", "7"), ("retry-after-ms", "250")]);
        assert_eq!(parse_retry_after(&h, NOW_MS), Some(250));
    }

    #[test]
    fn an_http_date_becomes_the_remaining_wait() {
        let h = headers(&[("retry-after", "Fri, 18 Sep 2026 00:00:30 GMT")]);
        assert_eq!(parse_retry_after(&h, NOW_MS), Some(30_000));
    }

    #[test]
    fn a_past_date_is_zero_rather_than_negative() {
        let h = headers(&[("retry-after", "Thu, 17 Sep 2026 00:00:00 GMT")]);
        assert_eq!(parse_retry_after(&h, NOW_MS), Some(0));
    }

    #[test]
    fn a_negative_delay_is_ignored() {
        assert_eq!(
            parse_retry_after(&headers(&[("retry-after", "-1")]), NOW_MS),
            None
        );
    }

    #[test]
    fn nonsense_is_ignored() {
        assert_eq!(
            parse_retry_after(&headers(&[("retry-after", "soon")]), NOW_MS),
            None
        );
    }

    #[test]
    fn absent_headers_yield_nothing() {
        assert_eq!(parse_retry_after(&Headers::new(), NOW_MS), None);
    }

    /// A malformed `retry-after-ms` must not shadow a usable `Retry-After`.
    #[test]
    fn a_broken_millisecond_header_falls_through() {
        let h = headers(&[("retry-after-ms", "soon"), ("retry-after", "2")]);
        assert_eq!(parse_retry_after(&h, NOW_MS), Some(2_000));
    }
}
