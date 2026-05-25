//! The [`ClientBuilder`].

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::client::Client;
use crate::error::{Error, Result};

/// The default base URL used when none is configured.
pub const DEFAULT_BASE_URL: &str = "http://localhost:3000";

/// The default request timeout applied when none is configured.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// The default number of retries for transient request failures.
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// A builder for [`Client`].
///
/// Obtain one via [`Client::builder`]. Each unset field falls back to its
/// `DEFAULT_*` constant.
///
/// ```no_run
/// use bentoml::prelude::*;
///
/// # fn run() -> Result<()> {
/// let client = Client::builder()
///     .with_base_url("http://localhost:3000")
///     .with_token("secret")
///     .build()?;
/// # let _ = client;
/// # Ok(())
/// # }
/// ```
///
/// [`Client::builder`]: crate::Client::builder
#[derive(Debug, Clone, Default)]
pub struct ClientBuilder {
    base_url: Option<String>,
    token: Option<String>,
    timeout: Option<Duration>,
    max_retries: Option<u32>,
    headers: Vec<(String, String)>,
}

impl ClientBuilder {
    /// Sets the base URL of the BentoML service, e.g. `http://localhost:3000`.
    ///
    /// Defaults to [`DEFAULT_BASE_URL`]. The URL is parsed when [`build`] is called.
    ///
    /// [`build`]: Self::build
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Sets the bearer token sent on each request. Unset by default.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Sets the per-request timeout. Defaults to [`DEFAULT_TIMEOUT`].
    pub fn with_timeout(mut self, timeout: impl Into<Duration>) -> Self {
        self.timeout = Some(timeout.into());
        self
    }

    /// Sets the maximum number of times a transient request failure is retried with
    /// exponential backoff. Defaults to [`DEFAULT_MAX_RETRIES`]; set to `0` to
    /// disable retries.
    pub fn with_max_retries(mut self, max_retries: impl Into<u32>) -> Self {
        self.max_retries = Some(max_retries.into());
        self
    }

    /// Adds a custom header sent on every request.
    ///
    /// May be called multiple times; the name and value are validated when [`build`]
    /// is called. Note that `Authorization` is managed by [`with_token`].
    ///
    /// [`build`]: Self::build
    /// [`with_token`]: Self::with_token
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Builds the [`Client`], consuming the builder.
    ///
    /// Returns an error if the configured base URL cannot be parsed, a custom header
    /// is invalid, or the HTTP client cannot be constructed.
    pub fn build(self) -> Result<Client> {
        let base_url = self.base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_owned());
        let timeout = self.timeout.unwrap_or(DEFAULT_TIMEOUT);
        let max_retries = self.max_retries.unwrap_or(DEFAULT_MAX_RETRIES);

        let mut headers = HeaderMap::with_capacity(self.headers.len());
        for (name, value) in self.headers {
            let name = name
                .parse::<HeaderName>()
                .map_err(|e| Error::InvalidHeader(format!("{name:?}: {e}")))?;
            let value = value
                .parse::<HeaderValue>()
                .map_err(|e| Error::InvalidHeader(format!("{name}: {e}")))?;
            headers.insert(name, value);
        }

        Client::assemble(base_url, self.token, timeout, max_retries, headers)
    }
}
