//! Client configuration, validation and precedence.
//!
//! One rule decides every setting: an explicit argument wins, then the
//! environment, then the SDK default. Everything is resolved and validated once
//! at construction, so a request in flight cannot be changed underneath and a
//! misconfigured client fails before it opens a socket.

mod build;
mod config;

pub use build::Builder;
pub use config::{Config, DEFAULT_BASE_URL, DEFAULT_MODEL, Parts};
