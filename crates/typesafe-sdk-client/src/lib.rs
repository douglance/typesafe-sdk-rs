//! The TypeSafe client.
//!
//! Three guarantees hold across every call here. The SDK's own headers are
//! written last, so a caller cannot replace `Authorization` or forge the retry
//! count. A question set is checked before anything is sent, so a malformed one
//! costs no round trip. And the per-attempt timeout covers the full response
//! body, not just the headers, so a server that stalls mid-body times out
//! rather than hanging.

mod attempt;
mod client;
mod headers;
mod options;
mod request;
mod send;

pub use client::Client;
pub use options::{RequestOptions, Responded};
pub use request::SystemOneRequest;

/// The SDK version, sent as `User-Agent` and `X-TypeSafe-SDK`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
