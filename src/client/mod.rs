//! The async HTTP client and its builder.

mod builder;

use std::sync::Arc;
use std::time::Duration;

use reqwest::Response;
use reqwest_middleware::{
    ClientBuilder as MiddlewareBuilder, ClientWithMiddleware, RequestBuilder,
};
use reqwest_retry::RetryTransientMiddleware;
use reqwest_retry::policies::ExponentialBackoff;
use serde::Serialize;
use serde::de::DeserializeOwned;
use url::Url;

pub use self::builder::{ClientBuilder, DEFAULT_BASE_URL, DEFAULT_MAX_RETRIES, DEFAULT_TIMEOUT};
use crate::error::{Error, Result};

/// An async client for a single BentoML service.
///
/// Construct one with [`Client::builder`], then invoke service endpoints with
/// [`Client::call`]. The client is cheap to clone: internally it is an
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
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, payload), err))]
    pub async fn call<T, R>(&self, route: &str, payload: &T) -> Result<R>
    where
        T: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let req = self.post(route)?.json(payload);
        let resp = self.send(req).await?;
        Ok(resp.json::<R>().await?)
    }

    /// Joins a route onto the base URL, tolerating an optional leading slash.
    pub(crate) fn endpoint(&self, route: &str) -> Result<Url> {
        let route = route.trim_start_matches('/');
        Ok(self.inner.base_url.join(route)?)
    }

    /// Like [`endpoint`], but appends a single query parameter.
    ///
    /// [`endpoint`]: Self::endpoint
    pub(crate) fn endpoint_query(&self, route: &str, key: &str, value: &str) -> Result<Url> {
        let mut url = self.endpoint(route)?;
        url.query_pairs_mut().append_pair(key, value);
        Ok(url)
    }

    /// Begins a `POST` request to `route`, with the bearer token applied.
    pub(crate) fn post(&self, route: &str) -> Result<RequestBuilder> {
        Ok(self.post_url(self.endpoint(route)?))
    }

    /// Begins a `GET` request to `route`, with the bearer token applied.
    pub(crate) fn get(&self, route: &str) -> Result<RequestBuilder> {
        Ok(self.get_url(self.endpoint(route)?))
    }

    /// Begins a `POST` request to a pre-built URL, with the bearer token applied.
    pub(crate) fn post_url(&self, url: Url) -> RequestBuilder {
        self.authed(self.inner.http.post(url))
    }

    /// Begins a `GET` request to a pre-built URL, with the bearer token applied.
    pub(crate) fn get_url(&self, url: Url) -> RequestBuilder {
        self.authed(self.inner.http.get(url))
    }

    /// Begins a `PUT` request to a pre-built URL, with the bearer token applied.
    pub(crate) fn put_url(&self, url: Url) -> RequestBuilder {
        self.authed(self.inner.http.put(url))
    }

    /// Applies the configured bearer token to a request builder, if any.
    pub(crate) fn authed(&self, req: RequestBuilder) -> RequestBuilder {
        match &self.inner.token {
            Some(token) => req.bearer_auth(token),
            None => req,
        }
    }

    /// Sends a request, mapping any non-success status to [`Error::Service`].
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, err))]
    pub(crate) async fn send(&self, req: RequestBuilder) -> Result<Response> {
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::service(status.as_u16(), &body));
        }
        Ok(resp)
    }

    /// Issues a `GET` against a health endpoint, returning whether it is healthy.
    pub(crate) async fn health(&self, route: &str) -> Result<bool> {
        let resp = self.get(route)?.send().await?;
        Ok(resp.status().is_success())
    }

    /// Assembles a [`Client`] from resolved configuration, used by [`ClientBuilder`].
    ///
    /// Builds the HTTP middleware stack: a per-request timeout enforced by reqwest,
    /// with retries layered on top via reqwest-middleware so each attempt gets its
    /// own timeout.
    pub(crate) fn assemble(
        base_url: String,
        token: Option<String>,
        timeout: Duration,
        max_retries: u32,
        headers: reqwest::header::HeaderMap,
    ) -> Result<Self> {
        let inner = reqwest::Client::builder()
            .timeout(timeout)
            .default_headers(headers)
            .build()?;

        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(max_retries);

        let http = MiddlewareBuilder::new(inner)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        let mut base_url = Url::parse(&base_url)?;

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
                token,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client(base_url: &str) -> Client {
        Client::builder().with_base_url(base_url).build().unwrap()
    }

    #[test]
    fn base_url_gets_a_trailing_slash() {
        // Without normalization, `Url::join` would drop the final path segment.
        assert_eq!(
            client("http://localhost:3000").base_url().as_str(),
            "http://localhost:3000/"
        );
        assert_eq!(
            client("http://localhost:3000/").base_url().as_str(),
            "http://localhost:3000/"
        );
    }

    #[test]
    fn endpoint_joins_route_tolerating_leading_slash() {
        let c = client("http://localhost:3000");
        assert_eq!(
            c.endpoint("summarize").unwrap().as_str(),
            "http://localhost:3000/summarize"
        );
        assert_eq!(
            c.endpoint("/summarize").unwrap().as_str(),
            "http://localhost:3000/summarize"
        );
    }

    #[test]
    fn endpoint_preserves_a_path_prefix() {
        // A `/v1/` prefix must survive route joining (BentoML `path_prefix`).
        let c = client("http://localhost:3000/v1");
        assert_eq!(
            c.endpoint("summarize").unwrap().as_str(),
            "http://localhost:3000/v1/summarize"
        );
    }
}
