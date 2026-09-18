//! The retry policy, backoff and delay calculation.

mod delay;
mod policy;

pub use delay::{Random, delay_ms};
pub use policy::{DEFAULT_MAX_RETRIES, DEFAULT_TIMEOUT_MS, RetryPolicy};
