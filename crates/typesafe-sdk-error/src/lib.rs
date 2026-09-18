//! The TypeSafe error taxonomy.
//!
//! The JavaScript SDK expresses this as a class hierarchy. Rust has no
//! inheritance, so the two relationships that callers actually branch on are
//! kept as predicates instead: [`Error::is_connection`] is true for a timeout
//! as well as a socket failure, and [`Error::status`] answers for any HTTP
//! error regardless of which status it carried.

mod api;
mod body;
mod describe;

pub use api::{ApiError, ApiErrorKind};
pub use body::Body;

use typesafe_sdk_headers::Headers;

/// Anything the SDK can fail with.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The API returned a non-2xx response.
    #[error("{0}")]
    Api(#[from] ApiError),

    /// The request or its response body could not be delivered.
    #[error("{message}")]
    Connection {
        /// What went wrong, including the underlying cause.
        message: String,
    },

    /// The full response did not arrive within the timeout.
    #[error("Request timed out after {timeout_ms}ms.")]
    Timeout {
        /// The timeout that elapsed, in milliseconds.
        timeout_ms: u64,
    },

    /// The caller cancelled the request.
    #[error("{message}")]
    UserAbort {
        /// Why the request was abandoned.
        message: String,
    },

    /// Configuration or a question was rejected before any request was sent.
    #[error("{0}")]
    Invalid(String),
}

impl Error {
    /// A connection failure, quoting the underlying cause as the JS SDK does.
    #[must_use]
    pub fn connection(cause: &str) -> Self {
        Self::Connection {
            message: format!("Connection error: {cause}"),
        }
    }

    /// The caller cancelled the request.
    #[must_use]
    pub fn aborted() -> Self {
        Self::UserAbort {
            message: "Request was aborted.".to_owned(),
        }
    }

    /// Builds the right variant for an HTTP status.
    #[must_use]
    pub fn from_response(status: u16, body: Body, headers: &Headers) -> Self {
        Self::Api(ApiError::from_response(status, body, headers))
    }

    /// The HTTP status, when this came from a response.
    #[must_use]
    pub const fn status(&self) -> Option<u16> {
        match self {
            Self::Api(error) => Some(error.status),
            _ => None,
        }
    }

    /// Whether this is a delivery failure.
    ///
    /// A timeout answers `true`, mirroring `APITimeoutError extends
    /// APIConnectionError`; retry policy distinguishes the two separately.
    #[must_use]
    pub const fn is_connection(&self) -> bool {
        matches!(self, Self::Connection { .. } | Self::Timeout { .. })
    }

    /// Whether the caller cancelled. Never retried.
    #[must_use]
    pub const fn is_user_abort(&self) -> bool {
        matches!(self, Self::UserAbort { .. })
    }

    /// The API error, when this came from a response.
    #[must_use]
    pub const fn api(&self) -> Option<&ApiError> {
        match self {
            Self::Api(error) => Some(error),
            _ => None,
        }
    }
}

/// The SDK result type.
pub type Result<T> = std::result::Result<T, Error>;
