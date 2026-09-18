//! A transport that answers from a script, for testing the retry loop.
//!
//! Every request is recorded, so a test can assert what went on the wire —
//! which headers were sent, in what order, and how many attempts were made —
//! without a socket or a timing dependency.

use std::sync::Mutex;

use async_trait::async_trait;
use typesafe_sdk_error::{Error, Result};
use typesafe_sdk_headers::Headers;

use crate::request::{RawResponse, Request};
use crate::transport::Transport;

/// One scripted answer.
pub enum Exchange {
    /// Answer with this status and body.
    Respond {
        /// The status to answer with.
        status: u16,
        /// The body to answer with.
        body: String,
        /// Headers to answer with.
        headers: Headers,
    },
    /// Fail the attempt without a response.
    Fail(Error),
}

impl Exchange {
    /// A successful JSON answer.
    #[must_use]
    pub fn ok(body: &str) -> Self {
        Self::status(200, body)
    }

    /// An answer with a given status.
    #[must_use]
    pub fn status(status: u16, body: &str) -> Self {
        Self::Respond {
            status,
            body: body.to_owned(),
            headers: Headers::new(),
        }
    }

    /// An answer carrying one header, such as `Retry-After`.
    #[must_use]
    pub fn with_header(self, name: &str, value: &str) -> Self {
        match self {
            Self::Respond {
                status,
                body,
                headers,
            } => Self::Respond {
                status,
                body,
                headers: headers.with(name, value),
            },
            other @ Self::Fail(_) => other,
        }
    }
}

/// A transport answering a fixed script, in order.
pub struct Mock {
    script: Mutex<Vec<Exchange>>,
    seen: Mutex<Vec<Request>>,
}

impl Mock {
    /// Builds a transport answering `script`, oldest first.
    #[must_use]
    pub fn new(script: Vec<Exchange>) -> Self {
        let mut script = script;
        script.reverse();
        Self {
            script: Mutex::new(script),
            seen: Mutex::new(Vec::new()),
        }
    }

    /// Every request that was attempted, in order.
    #[must_use]
    pub fn requests(&self) -> Vec<Request> {
        self.seen
            .lock()
            .map_or_else(|error| error.into_inner().clone(), |seen| seen.clone())
    }

    /// How many attempts were made.
    #[must_use]
    pub fn attempts(&self) -> usize {
        self.requests().len()
    }
}

#[async_trait]
impl Transport for Mock {
    async fn send(&self, request: Request) -> Result<RawResponse> {
        if let Ok(mut seen) = self.seen.lock() {
            seen.push(request);
        }
        let next = self
            .script
            .lock()
            .ok()
            .and_then(|mut script| script.pop())
            .ok_or_else(|| Error::Invalid("the mock transport ran out of answers".to_owned()))?;

        match next {
            Exchange::Respond {
                status,
                body,
                headers,
            } => Ok(RawResponse {
                status,
                headers,
                body,
            }),
            Exchange::Fail(error) => Err(error),
        }
    }
}
