//! The async HTTP client and its builder.

mod config;

use std::sync::Arc;

use reqwest_middleware::{ClientBuilder as MiddlewareBuilder, ClientWithMiddleware};
use reqwest_retry::RetryTransientMiddleware;
use reqwest_retry::policies::ExponentialBackoff;
use serde::Serialize;
use serde::de::DeserializeOwned;
use url::Url;

pub use self::config::{
    ClientBuilder, ClientBuilderError, ClientConfig, DEFAULT_BASE_URL, DEFAULT_MAX_RETRIES,
    DEFAULT_TIMEOUT,
};
use crate::error::{Error, Result};

/// An async client for a single BentoML service.
///
/// Construct one with [`Client::builder`], then invoke service endpoints with
/// [`Client::call`]. The client is cheap to clone — internally it is an
/// [`Arc`] around shared state, so clones share one connection pool. Requests pass
/// through a [`reqwest-middleware`] stack that applies a per-request timeout and
/// retries transient failures with exponential backoff.
///
/// [`reqwest-middleware`]: https://docs.rs/reqwest-middleware
#[derive(Debug, Clone)]
pub struct Client {
    inner: Arc<ClientImpl>,
}

/// The shared state behind a [`Client`], held by an [`Arc`] so clones are cheap.
#[derive(Debug)]
struct ClientImpl {
    http: ClientWithMiddleware,
    base_url: Url,
    token: Option<String>,
}

impl Client {
    /// Returns a new [`ClientBuilder`].
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// The base URL this client targets.
    pub fn base_url(&self) -> &Url {
        &self.inner.base_url
    }

    /// Invokes the endpoint at `route` with the given JSON `payload`, returning
    /// the deserialized response.
    ///
    /// `route` is the endpoint name with or without a leading slash; it is joined
    /// onto the configured base URL. BentoML endpoints are `POST` by default.
    pub async fn call<T, R>(&self, route: &str, payload: &T) -> Result<R>
    where
        T: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let url = self.endpoint(route)?;
        let mut req = self.inner.http.post(url).json(payload);
        if let Some(token) = &self.inner.token {
            req = req.bearer_auth(token);
        }

        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(Error::service(status.as_u16(), message));
        }

        Ok(resp.json::<R>().await?)
    }

    /// Returns whether the service reports itself ready, via the `/readyz`
    /// health-check endpoint.
    pub async fn is_ready(&self) -> Result<bool> {
        let url = self.endpoint("readyz")?;
        let resp = self.inner.http.get(url).send().await?;
        Ok(resp.status().is_success())
    }

    /// Joins a route onto the base URL, tolerating an optional leading slash.
    fn endpoint(&self, route: &str) -> Result<Url> {
        let route = route.trim_start_matches('/');
        Ok(self.inner.base_url.join(route)?)
    }

    /// Builds a [`Client`] from a resolved [`ClientConfig`].
    ///
    /// Assembles the HTTP middleware stack: a per-request timeout enforced by
    /// reqwest, with retries layered on top via reqwest-middleware so each attempt
    /// gets its own timeout.
    pub(crate) fn from_config(config: ClientConfig) -> Result<Self> {
        let inner = reqwest::Client::builder().timeout(config.timeout).build()?;

        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(config.max_retries);

        let http = MiddlewareBuilder::new(inner)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        let mut base_url = Url::parse(&config.base_url)?;

        // A base URL must end in `/` for [`Url::join`] to treat it as a directory
        // rather than replacing the final path segment.
        if !base_url.path().ends_with('/') {
            let path = format!("{}/", base_url.path());
            base_url.set_path(&path);
        }

        Ok(Self {
            inner: Arc::new(ClientImpl {
                http,
                base_url,
                token: config.token,
            }),
        })
    }
}
