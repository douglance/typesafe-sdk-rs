//! What goes out, and what comes back.

use typesafe_sdk_headers::Headers;

/// The methods this API uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Retrieval, with no body.
    Get,
    /// Submission, with a JSON body.
    Post,
}

impl Method {
    /// The method name as it appears on the wire and in logs.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// One attempt at one request.
#[derive(Debug, Clone)]
pub struct Request {
    /// The method.
    pub method: Method,
    /// The absolute URL.
    pub url: String,
    /// Every header, already merged and with the SDK's own applied last.
    pub headers: Headers,
    /// The serialised body, when there is one.
    pub body: Option<String>,
    /// How long this attempt may take, including reading the body.
    pub timeout_ms: u64,
}

/// A response, with its body already read.
#[derive(Debug, Clone)]
pub struct RawResponse {
    /// The HTTP status.
    pub status: u16,
    /// The response headers.
    pub headers: Headers,
    /// The body as text, empty when there was none.
    pub body: String,
}

impl RawResponse {
    /// Whether the status is in the 2xx range.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}
