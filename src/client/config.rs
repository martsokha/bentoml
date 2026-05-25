//! Client configuration.

use std::time::Duration;

use derive_builder::Builder;

use crate::client::Client;
use crate::error::{Error, Result};

/// The default base URL used when none is configured.
pub const DEFAULT_BASE_URL: &str = "http://localhost:3000";

/// The default request timeout applied when none is configured.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// The default number of retries for transient request failures.
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// Resolved configuration for a [`Client`](crate::Client).
///
/// This is an implementation detail. Construct a client through
/// [`Client::builder`](crate::Client::builder), which yields a
/// [`ClientBuilder`](crate::ClientBuilder).
#[doc(hidden)]
#[derive(Debug, Clone, Builder)]
#[builder(
    name = "ClientBuilder",
    pattern = "owned",
    setter(into, strip_option, prefix = "with"),
    build_fn(private, name = "build_config"),
    derive(Debug, Clone)
)]
pub struct ClientConfig {
    /// The base URL of the BentoML service, e.g. `http://localhost:3000`.
    #[builder(default = "DEFAULT_BASE_URL.to_owned()")]
    pub base_url: String,
    /// An optional bearer token sent on each request.
    #[builder(default)]
    pub token: Option<String>,
    /// The per-request timeout.
    #[builder(default = "DEFAULT_TIMEOUT")]
    pub timeout: Duration,
    /// The maximum number of retries for transient request failures.
    #[builder(default = "DEFAULT_MAX_RETRIES")]
    pub max_retries: u32,
}

impl Default for ClientConfig {
    fn default() -> Self {
        ClientBuilder::default()
            .build_config()
            .expect("all ClientConfig fields have defaults")
    }
}

impl ClientBuilder {
    /// Builds the [`Client`], consuming the builder.
    ///
    /// Returns an error if the configured base URL cannot be parsed or the HTTP
    /// client cannot be constructed.
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
    pub fn build(self) -> Result<Client> {
        let config = self
            .build_config()
            .map_err(|e| Error::Builder(e.to_string()))?;
        Client::from_config(config)
    }
}
