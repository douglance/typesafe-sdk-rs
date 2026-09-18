//! The retry policy, backoff and delay calculation.
//!
//! Two properties here are easy to get subtly wrong and are pinned by tests.
//! Jitter only ever subtracts, so a delay never exceeds its nominal backoff.
//! And a `Retry-After` the policy accepts is honoured exactly, with no jitter
//! at all, because the server named a time and second-guessing it helps nobody.

mod delay;
mod policy;

pub use delay::{Random, Wait, delay_ms};
pub use policy::{DEFAULT_MAX_RETRIES, DEFAULT_TIMEOUT_MS, RetryPolicy};
