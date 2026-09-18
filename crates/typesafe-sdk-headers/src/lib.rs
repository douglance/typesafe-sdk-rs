//! Case-insensitive header merging and secret redaction.
//!
//! Two behaviours the SDK depends on and a plain map does not give you: names
//! compare case-insensitively while the last writer's spelling is what goes on
//! the wire, and an explicit "no value" removes a header rather than setting it
//! to empty. That second rule is how the client guarantees a caller cannot
//! clobber `Authorization` or attach a `Content-Type` to a request with no body.

mod merge;
mod redact;
mod retry_after;

pub use merge::{Headers, Value};
pub use redact::redact;
pub use retry_after::parse_retry_after;
