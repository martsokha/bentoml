//! The [`ClientBuilder`].

use std::time::Duration;

use url::Url;

use crate::client::Client;
use crate::client::config::DEFAULT_TIMEOUT;
use crate::error::{Error, Result};

/// A builder for [`Client`].
///
/// ```no_run
/// use bentoml::prelude::*;
///
/// # fn run() -> Result<()> {
/// let client = Client::builder()
///     .base_url("http://localhost:3000")?
///     .token("secret")
///     .build()?;
/// # let _ = client;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default)]
pub struct ClientBuilder {
    base_url: Option<Url>,
    token: Option<String>,
    timeout: Option<Duration>,
}

impl ClientBuilder {
    /// Sets the base URL of the BentoML service.
    ///
    /// Returns an error immediately if the URL cannot be parsed.
    pub fn base_url(mut self, url: impl AsRef<str>) -> Result<Self> {
        self.base_url = Some(Url::parse(url.as_ref())?);
        Ok(self)
    }

    /// Sets the bearer token sent on each request.
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Overrides the per-request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Builds the [`Client`], consuming the builder.
    pub fn build(self) -> Result<Client> {
        let mut base_url = self
            .base_url
            .ok_or_else(|| Error::Builder("base_url is required".to_owned()))?;

        // A base URL must end in `/` for [`Url::join`] to treat it as a directory
        // rather than replacing the final path segment.
        if !base_url.path().ends_with('/') {
            let path = format!("{}/", base_url.path());
            base_url.set_path(&path);
        }

        let http = reqwest::Client::builder()
            .timeout(self.timeout.unwrap_or(DEFAULT_TIMEOUT))
            .build()?;

        Ok(Client::from_parts(http, base_url, self.token))
    }
}
