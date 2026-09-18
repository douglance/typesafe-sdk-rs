//! The client itself.

use std::sync::Arc;

use typesafe_sdk_answers::SystemOneResponse;
use typesafe_sdk_config::Config;
use typesafe_sdk_error::{Error, Result};
use typesafe_sdk_headers::Headers;
use typesafe_sdk_http::{Method, RawResponse, Request, Transport};
use typesafe_sdk_log::Level;
use typesafe_sdk_models::{ModelCard, unwrap_listing};
use typesafe_sdk_questions::validate;

use crate::attempt;
use crate::headers::assemble;
use crate::request::SystemOneRequest;

/// Where the models listing lives.
const MODELS_PATH: &str = "/v1/models";

/// Where System One lives.
const SYSTEM_ONE_PATH: &str = "/v1/systemone";

/// A configured TypeSafe client.
pub struct Client {
    config: Config,
    transport: Arc<dyn Transport>,
}

impl Client {
    /// Builds a client over a given transport.
    #[must_use]
    pub fn with_transport(config: Config, transport: Arc<dyn Transport>) -> Self {
        Self { config, transport }
    }

    /// The resolved settings this client runs on.
    #[must_use]
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// Lists the models available to the account.
    ///
    /// # Errors
    /// Returns an API, connection or timeout error, or [`Error::Invalid`] if
    /// the response is not the documented envelope.
    pub async fn models(&self) -> Result<Vec<ModelCard>> {
        let response = self.send(Method::Get, MODELS_PATH, None).await?;
        let body: serde_json::Value = serde_json::from_str(&response.body).map_err(|_| {
            Error::Invalid(
                "Unexpected response shape from GET /v1/models; expected { models: [...] }."
                    .to_owned(),
            )
        })?;
        unwrap_listing(&body)
    }

    /// Answers `request`'s questions about its state.
    ///
    /// Questions are checked before anything is sent, so a malformed set costs
    /// no round trip and reports in the caller's terms.
    ///
    /// # Errors
    /// Returns [`Error::Invalid`] for a malformed question set, or an API,
    /// connection or timeout error.
    pub async fn system_one(&self, request: SystemOneRequest) -> Result<SystemOneResponse> {
        validate(&request.questions)?;
        let mut request = request;
        request.resolve_model(&self.config.default_model);

        let body = serde_json::to_string(&request)
            .map_err(|error| Error::Invalid(format!("could not serialise the request: {error}")))?;
        let response = self.send(Method::Post, SYSTEM_ONE_PATH, Some(body)).await?;

        serde_json::from_str(&response.body).map_err(|error| {
            Error::Invalid(format!(
                "Unexpected response shape from POST /v1/systemone: {error}."
            ))
        })
    }

    /// Runs one request through the retry loop.
    async fn send(&self, method: Method, path: &str, body: Option<String>) -> Result<RawResponse> {
        let url = format!("{}{path}", self.config.base_url);
        let tag = format!("{method} {path}");
        self.config
            .logger
            .log(Level::Debug, || format!("{tag} -> {url}"));

        let response = attempt::run(self.transport.as_ref(), &self.config, &tag, |attempt| {
            Request {
                method,
                url: url.clone(),
                headers: assemble(&self.config, &Headers::new(), body.is_some(), attempt),
                body: body.clone(),
                timeout_ms: self.config.timeout_ms,
            }
        })
        .await?;

        self.config
            .logger
            .log(Level::Info, || format!("{tag} <- {}", response.status));
        Ok(response)
    }
}
