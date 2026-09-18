//! The client itself.

use std::sync::Arc;

use typesafe_sdk_answers::SystemOneResponse;
use typesafe_sdk_config::Config;
use typesafe_sdk_error::{Error, Result};
use typesafe_sdk_http::{Method, RawResponse, Transport};
use typesafe_sdk_log::Level;
use typesafe_sdk_models::{ModelCard, unwrap_listing};
use typesafe_sdk_questions::validate;

use crate::options::{RequestOptions, Responded};
use crate::request::SystemOneRequest;
use crate::send::{Call, send};

/// Where the models listing lives.
const MODELS_PATH: &str = "/v1/models";

/// Where System One lives.
const SYSTEM_ONE_PATH: &str = "/v1/systemone";

/// The header the service answers request identifiers in.
const REQUEST_ID: &str = "x-typesafe-request-id";

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
        self.models_with(&RequestOptions::new())
            .await
            .map(Responded::into_data)
    }

    /// Lists the models, with per-call overrides and response metadata.
    ///
    /// # Errors
    /// As [`Client::models`].
    pub async fn models_with(&self, options: &RequestOptions) -> Result<Responded<Vec<ModelCard>>> {
        let response = self
            .call(&Call {
                method: Method::Get,
                path: MODELS_PATH,
                body: None,
                options,
            })
            .await?;
        let body: serde_json::Value = serde_json::from_str(&response.body).map_err(|_| {
            Error::Invalid(
                "Unexpected response shape from GET /v1/models; expected { models: [...] }."
                    .to_owned(),
            )
        })?;
        Ok(responded(&response, unwrap_listing(&body)?))
    }

    /// Answers `request`'s questions about its state.
    ///
    /// # Errors
    /// Returns [`Error::Invalid`] for a malformed question set, or an API,
    /// connection or timeout error.
    pub async fn system_one(&self, request: SystemOneRequest) -> Result<SystemOneResponse> {
        self.system_one_with(request, &RequestOptions::new())
            .await
            .map(Responded::into_data)
    }

    /// Answers a request, with per-call overrides and response metadata.
    ///
    /// Questions are checked before anything is sent, so a malformed set costs
    /// no round trip and reports in the caller's terms.
    ///
    /// # Errors
    /// As [`Client::system_one`].
    pub async fn system_one_with(
        &self,
        request: SystemOneRequest,
        options: &RequestOptions,
    ) -> Result<Responded<SystemOneResponse>> {
        validate(&request.questions)?;
        let mut request = request;
        request.resolve_model(&self.config.default_model);

        let body = serde_json::to_string(&request)
            .map_err(|error| Error::Invalid(format!("could not serialise the request: {error}")))?;
        let response = self
            .call(&Call {
                method: Method::Post,
                path: SYSTEM_ONE_PATH,
                body: Some(body),
                options,
            })
            .await?;

        let parsed = serde_json::from_str(&response.body).map_err(|error| {
            Error::Invalid(format!(
                "Unexpected response shape from POST /v1/systemone: {error}."
            ))
        })?;
        Ok(responded(&response, parsed))
    }

    /// Runs one request through the retry loop.
    async fn call(&self, call: &Call<'_>) -> Result<RawResponse> {
        let response = send(self.transport.as_ref(), &self.config, call).await?;
        self.config.logger.log(Level::Info, || {
            format!("{} {} <- {}", call.method, call.path, response.status)
        });
        Ok(response)
    }
}

/// Attaches the response metadata a caller may need for support.
fn responded<T>(response: &RawResponse, data: T) -> Responded<T> {
    Responded {
        data,
        status: response.status,
        request_id: response.headers.get(REQUEST_ID).map(ToOwned::to_owned),
    }
}
