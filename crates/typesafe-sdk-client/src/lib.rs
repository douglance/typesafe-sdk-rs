//! The TypeSafe client.

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
