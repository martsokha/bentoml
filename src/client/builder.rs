//! The [`ClientBuilder`].

use std::time::Duration;

use crate::client::{Client, Headers};
use crate::error::Result;

/// A builder for [`Client`].
///
/// Obtain one via [`Client::builder`]. Each unset field falls back to a default,
/// noted on the corresponding setter.
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
    headers: Headers,
}

impl ClientBuilder {
    /// The base URL used when none is configured.
    const DEFAULT_BASE_URL: &str = "http://localhost:3000";
    /// The number of retries for transient failures used when none is configured.
    const DEFAULT_MAX_RETRIES: u32 = 3;

    /// Sets the base URL of the BentoML service, e.g. `http://localhost:3000`.
    ///
    /// Defaults to `http://localhost:3000`. The URL is parsed when [`build`] is
    /// called.
    ///
    /// [`build`]: Self::build
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Sets the bearer token sent on each request, as `Authorization: Bearer <token>`.
    /// Unset by default.
    ///
    /// This is the scheme BentoML uses. For a non-bearer scheme, set the header
    /// directly with [`with_header`] (e.g. `with_header("authorization", "Basic ...")`).
    ///
    /// [`with_header`]: Self::with_header
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Sets the per-request timeout. Unset by default (no timeout), matching
    /// reqwest; suited to long-running inference, tasks, and streaming.
    pub fn with_timeout(mut self, timeout: impl Into<Duration>) -> Self {
        self.timeout = Some(timeout.into());
        self
    }

    /// Sets the maximum number of times a transient request failure is retried with
    /// exponential backoff. Defaults to `3`; set to `0` to disable retries.
    pub fn with_max_retries(mut self, max_retries: impl Into<u32>) -> Self {
        self.max_retries = Some(max_retries.into());
        self
    }

    /// Adds a custom header sent on every request.
    ///
    /// May be called multiple times; an invalid name or value is reported when
    /// [`build`] is called. Note that `Authorization` is managed by [`with_token`].
    ///
    /// [`build`]: Self::build
    /// [`with_token`]: Self::with_token
    pub fn with_header(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Sets the `User-Agent` header sent on every request.
    ///
    /// Convenience for [`with_header`] with the `User-Agent` name.
    ///
    /// [`with_header`]: Self::with_header
    pub fn with_user_agent(self, value: impl AsRef<str>) -> Self {
        self.with_header("user-agent", value)
    }

    /// Builds the [`Client`], consuming the builder.
    ///
    /// Returns an error if the configured base URL cannot be parsed, a custom header
    /// is invalid, or the HTTP client cannot be constructed.
    pub fn build(self) -> Result<Client> {
        let base_url = self
            .base_url
            .unwrap_or_else(|| Self::DEFAULT_BASE_URL.to_owned());
        let max_retries = self.max_retries.unwrap_or(Self::DEFAULT_MAX_RETRIES);
        let headers = self.headers.into_map()?;

        Client::assemble(base_url, self.token, self.timeout, max_retries, headers)
    }
}
