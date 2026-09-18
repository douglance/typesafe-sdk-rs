//! A non-2xx response, classified.

use typesafe_sdk_headers::{Headers, parse_retry_after};

use crate::body::Body;
use crate::describe::describe;

/// The header the API answers request identifiers in.
const REQUEST_ID: &str = "x-typesafe-request-id";

/// Which kind of failure a status code represents.
///
/// These are the cases callers branch on. Anything unrecognised stays
/// [`ApiErrorKind::Other`] rather than being forced into a neighbour, so a new
/// status the API starts returning is visible instead of silently mislabelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiErrorKind {
    /// 400: the request is invalid.
    BadRequest,
    /// 401: authentication failed.
    Authentication,
    /// 403: access is denied.
    PermissionDenied,
    /// 404: the resource was not found.
    NotFound,
    /// 422: request validation failed.
    UnprocessableEntity,
    /// 429: the rate limit was exceeded.
    RateLimit,
    /// 5xx: the server failed to handle the request.
    InternalServer,
    /// Any other status.
    Other,
}

/// Statuses the API gives a distinct meaning, as data rather than branches.
const MAPPED: &[(u16, ApiErrorKind)] = &[
    (400, ApiErrorKind::BadRequest),
    (401, ApiErrorKind::Authentication),
    (403, ApiErrorKind::PermissionDenied),
    (404, ApiErrorKind::NotFound),
    (422, ApiErrorKind::UnprocessableEntity),
    (429, ApiErrorKind::RateLimit),
];

impl ApiErrorKind {
    /// Classifies an HTTP status exactly as the JS SDK does.
    ///
    /// Note the 5xx test has no upper bound, so a nonstandard 6xx status still
    /// reads as a server failure. That is the JS behaviour, kept deliberately.
    #[must_use]
    pub fn of(status: u16) -> Self {
        if let Some(&(_, kind)) = MAPPED.iter().find(|&&(code, _)| code == status) {
            return kind;
        }
        if status >= 500 {
            return Self::InternalServer;
        }
        Self::Other
    }
}

/// An unsuccessful HTTP response from the API.
#[derive(Debug, Clone, thiserror::Error)]
#[error("{message}")]
pub struct ApiError {
    /// HTTP status code.
    pub status: u16,
    /// Which failure this status represents.
    pub kind: ApiErrorKind,
    /// The rendered message, as the JS SDK renders it.
    pub message: String,
    /// The response body, parsed where possible.
    pub body: Body,
    /// Response headers.
    pub headers: Headers,
    /// Value of `x-typesafe-request-id`, when the API supplied one.
    pub request_id: Option<String>,
    /// For a rate limit, the delay the server asked for.
    ///
    /// Uncapped, unlike the retry policy's own reading of the same header: this
    /// reports what the server said, not what the client intends to do.
    pub retry_after_ms: Option<u64>,
}

impl ApiError {
    /// Builds the error for a response.
    #[must_use]
    pub fn from_response(status: u16, body: Body, headers: &Headers) -> Self {
        let kind = ApiErrorKind::of(status);
        Self {
            status,
            kind,
            message: describe(status, &body),
            body,
            request_id: headers.get(REQUEST_ID).map(ToOwned::to_owned),
            retry_after_ms: match kind {
                ApiErrorKind::RateLimit => parse_retry_after(headers, now_ms()),
                _ => None,
            },
            headers: headers.clone(),
        }
    }
}

/// Unix epoch milliseconds, for resolving an `Retry-After` date.
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|since| i64::try_from(since.as_millis()).ok())
        .unwrap_or(0)
}
