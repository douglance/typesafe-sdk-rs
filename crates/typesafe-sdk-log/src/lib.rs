//! Log levels, filtering and the console sink.
//!
//! The SDK never logs at `warn` or `error` itself: a failed request is returned
//! to the caller, and logging it too would double-report someone else's
//! decision. Everything the SDK emits is `debug` or `info`, which is why the
//! default level of `warn` makes a healthy client completely silent.

mod level;
mod sink;

pub use level::{DEFAULT_LEVEL, LEVELS, Level};
pub use sink::{Console, Filtered, Line, Logger, Recording, Silent};
