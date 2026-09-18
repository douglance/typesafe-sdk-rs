//! Sending a request over the network.

use std::time::Duration;

use async_trait::async_trait;
use typesafe_sdk_error::{Error, Result};
use typesafe_sdk_headers::Headers;

use crate::request::{Method, RawResponse, Request};

/// Somewhere a request can be sent.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Performs one attempt.
    ///
    /// Implementations must return [`Error::Timeout`] if the whole exchange,
    /// body included, exceeds `request.timeout_ms`.
    ///
    /// # Errors
    /// Returns a connection or timeout error; a non-2xx status is a successful
    /// send and is reported through [`RawResponse::status`].
    async fn send(&self, request: Request) -> Result<RawResponse>;
}

/// The real transport, over reqwest.
#[derive(Debug, Clone)]
pub struct Reqwest {
    client: reqwest::Client,
}

impl Reqwest {
    /// Builds a transport with its own connection pool.
    ///
    /// # Errors
    /// Returns [`Error::Connection`] if the TLS backend cannot be initialised.
    pub fn new() -> Result<Self> {
        reqwest::Client::builder()
            .build()
            .map(|client| Self { client })
            .map_err(|error| Error::connection(&error.to_string()))
    }
}

#[async_trait]
impl Transport for Reqwest {
    async fn send(&self, request: Request) -> Result<RawResponse> {
        let timeout = Duration::from_millis(request.timeout_ms);
        let timeout_ms = request.timeout_ms;
        // The timeout wraps reading the body as well as receiving the headers,
        // so a server that stalls mid-body is a timeout rather than a hang.
        tokio::time::timeout(timeout, self.exchange(request))
            .await
            .map_err(|_| Error::Timeout { timeout_ms })?
    }
}

impl Reqwest {
    async fn exchange(&self, request: Request) -> Result<RawResponse> {
        let response = self
            .build(&request)
            .send()
            .await
            .map_err(|error| Error::connection(&error.to_string()))?;

        let status = response.status().as_u16();
        let headers = collect_headers(response.headers());
        let body = response
            .text()
            .await
            .map_err(|error| Error::connection(&error.to_string()))?;
        Ok(RawResponse {
            status,
            headers,
            body,
        })
    }

    fn build(&self, request: &Request) -> reqwest::RequestBuilder {
        let builder = match request.method {
            Method::Get => self.client.get(&request.url),
            Method::Post => self.client.post(&request.url),
        };
        let builder = request
            .headers
            .iter()
            .fold(builder, |acc, (name, value)| acc.header(name, value));
        match request.body.as_ref() {
            Some(body) => builder.body(body.clone()),
            None => builder,
        }
    }
}

/// Copies response headers into the SDK's own map.
fn collect_headers(source: &reqwest::header::HeaderMap) -> Headers {
    source
        .iter()
        .filter_map(|(name, value)| value.to_str().ok().map(|v| (name.as_str(), v)))
        .fold(Headers::new(), |acc, (name, value)| acc.with(name, value))
}
