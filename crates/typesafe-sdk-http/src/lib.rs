//! The HTTP transport seam and its reqwest implementation.
//!
//! The seam exists so the retry loop, which is the part with interesting
//! behaviour, can be tested without a socket. The one subtlety the real
//! implementation must preserve is that the per-attempt timeout covers the
//! *full response body*, not just the headers: a server that sends headers
//! promptly and then stalls mid-body must produce a timeout, not a hang.

mod mock;
mod request;
mod transport;

pub use mock::{Exchange, Mock};
pub use request::{Method, RawResponse, Request};
pub use transport::{Reqwest, Transport};
