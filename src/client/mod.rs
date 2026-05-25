//! The async HTTP client and its builder.

mod builder;
mod config;

use serde::Serialize;
use serde::de::DeserializeOwned;
use url::Url;

pub use self::builder::ClientBuilder;
pub use self::config::Config;
use crate::error::{Error, Result};

/// An async client for a single BentoML service.
///
/// Construct one with [`Client::builder`], then invoke service endpoints with
/// [`Client::call`]. The client is cheap to clone — the underlying
/// [`reqwest::Client`] uses an internal connection pool shared across clones.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
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
        &self.base_url
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
        let mut req = self.http.post(url).json(payload);
        if let Some(token) = &self.token {
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
        let resp = self.http.get(url).send().await?;
        Ok(resp.status().is_success())
    }

    /// Joins a route onto the base URL, tolerating an optional leading slash.
    fn endpoint(&self, route: &str) -> Result<Url> {
        let route = route.trim_start_matches('/');
        Ok(self.base_url.join(route)?)
    }

    /// Internal constructor used by [`ClientBuilder`].
    pub(crate) fn from_parts(http: reqwest::Client, base_url: Url, token: Option<String>) -> Self {
        Self {
            http,
            base_url,
            token,
        }
    }
}
