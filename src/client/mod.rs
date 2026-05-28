//! The async HTTP client and its builder.

mod builder;
mod endpoint;
mod headers;
mod reply;

pub mod multipart;

use std::borrow::Cow;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::Response as ReqwestResponse;
use reqwest_middleware::{
    ClientBuilder as MiddlewareBuilder, ClientWithMiddleware, RequestBuilder,
};
use reqwest_retry::RetryTransientMiddleware;
use reqwest_retry::policies::ExponentialBackoff;
use url::Url;

pub use self::builder::ClientBuilder;
pub use self::endpoint::Endpoint;
pub(crate) use self::endpoint::EndpointBase;
pub(crate) use self::headers::Headers;
pub use self::reply::EndpointReply;
use crate::error::{Error, Result};

/// An async client for a single BentoML service.
///
/// Construct one with [`Client::builder`], then invoke service endpoints through an
/// [`Endpoint`] handle from [`Client::endpoint`] (synchronous `@bentoml.api`) or a
/// [`TaskEndpoint`] from [`Client::task`] (async `@bentoml.task`). The client is cheap
/// to clone: internally it is an [`Arc`] around shared state, so clones share one
/// connection pool. Requests pass through a [`reqwest-middleware`] stack that applies a
/// per-request timeout and retries transient failures with exponential backoff.
///
/// [`TaskEndpoint`]: crate::task::TaskEndpoint
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

    /// The underlying [`reqwest::Client`], for requests this crate doesn't model.
    ///
    /// A hidden escape hatch (`#[doc(hidden)]`, not part of the stable API): build any
    /// request against [`base_url`] and send it with the client's configured TLS,
    /// connection pool, and bearer token. This is the bare client, so it does *not*
    /// carry the retry middleware — use [`as_client_with_middleware`] to keep retries.
    ///
    /// [`base_url`]: Self::base_url
    /// [`as_client_with_middleware`]: Self::as_client_with_middleware
    #[doc(hidden)]
    pub fn as_client(&self) -> &reqwest::Client {
        self.inner.http.as_ref()
    }

    /// The underlying [`reqwest_middleware::ClientWithMiddleware`], for requests this
    /// crate doesn't model.
    ///
    /// A hidden escape hatch (`#[doc(hidden)]`, not part of the stable API) like
    /// [`as_client`], but with the retry middleware applied, so custom requests get
    /// the same exponential-backoff retries as the crate's own. The bearer token is
    /// carried via the client's default headers, like every other request.
    ///
    /// [`as_client`]: Self::as_client
    #[doc(hidden)]
    pub fn as_client_with_middleware(&self) -> &reqwest_middleware::ClientWithMiddleware {
        &self.inner.http
    }

    /// Returns a handle to the synchronous endpoint (`@bentoml.api`) at `route`.
    ///
    /// `route` is the endpoint name with or without a leading slash. The returned
    /// [`Endpoint`] carries the route, so calls are made on it rather than passing
    /// the route to each method: `client.endpoint("summarize").invoke(&req)`. For an
    /// async task endpoint (`@bentoml.task`), use [`task`] instead.
    ///
    /// Accepts a `&'static str` (borrowed without allocating) or an owned `String`.
    ///
    /// [`task`]: Self::task
    pub fn endpoint(&self, route: impl Into<Cow<'static, str>>) -> Endpoint {
        Endpoint::new(self.clone(), route.into())
    }

    /// Returns a handle to the async task endpoint (`@bentoml.task`) at `route`.
    ///
    /// The returned [`TaskEndpoint`] exposes only the task surface — `submit` /
    /// `submit_bytes` / `submit_multipart`, each returning a [`TaskHandle`] — so the
    /// synchronous `call` family is not callable on it. For a synchronous endpoint,
    /// use [`endpoint`] instead.
    ///
    /// [`TaskEndpoint`]: crate::task::TaskEndpoint
    /// [`TaskHandle`]: crate::task::TaskHandle
    /// [`endpoint`]: Self::endpoint
    pub fn task(&self, route: impl Into<Cow<'static, str>>) -> crate::task::TaskEndpoint {
        crate::task::TaskEndpoint::new(self.clone(), route.into())
    }

    /// Returns whether the service reports itself ready, via `/readyz`.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self), err))]
    pub async fn is_ready(&self) -> Result<bool> {
        self.health("readyz").await
    }

    /// Returns whether the service reports itself alive, via `/livez`.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self), err))]
    pub async fn is_live(&self) -> Result<bool> {
        self.health("livez").await
    }

    /// Polls [`is_ready`] until it returns `true` or `timeout` elapses, awaiting
    /// `sleep(interval)` between attempts.
    ///
    /// The crate is runtime-agnostic, so the caller supplies the delay: `sleep` is
    /// invoked with `interval` and the returned future is awaited. With Tokio, pass
    /// `tokio::time::sleep`.
    ///
    /// Returns [`Error::Timeout`] if the service does not become ready in time.
    ///
    /// [`is_ready`]: Self::is_ready
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, sleep), err))]
    pub async fn wait_until_ready<S, F>(
        &self,
        timeout: Duration,
        interval: Duration,
        mut sleep: S,
    ) -> Result<()>
    where
        S: FnMut(Duration) -> F + Send,
        F: Future<Output = ()> + Send,
    {
        let deadline = Instant::now() + timeout;
        loop {
            // Ignore transient errors (e.g. connection refused during startup);
            // only the deadline ends the loop.
            if matches!(self.is_ready().await, Ok(true)) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(Error::Timeout { timeout });
            }
            sleep(interval).await;
        }
    }

    /// Joins a route onto the base URL, tolerating an optional leading slash.
    pub(crate) fn route_url(&self, route: &str) -> Result<Url> {
        let route = route.trim_start_matches('/');
        Ok(self.inner.base_url.join(route)?)
    }

    /// Like [`route_url`], but appends a single query parameter.
    ///
    /// [`route_url`]: Self::route_url
    pub(crate) fn endpoint_query(&self, route: &str, key: &str, value: &str) -> Result<Url> {
        let mut url = self.route_url(route)?;
        url.query_pairs_mut().append_pair(key, value);
        Ok(url)
    }

    /// Begins a `POST` request to `route`.
    pub(crate) fn post(&self, route: &str) -> Result<RequestBuilder> {
        Ok(self.post_url(self.route_url(route)?))
    }

    /// Begins a `GET` request to `route`.
    pub(crate) fn get(&self, route: &str) -> Result<RequestBuilder> {
        Ok(self.get_url(self.route_url(route)?))
    }

    /// Begins a `POST` request to a pre-built URL.
    ///
    /// The bearer token (if any) is applied via the client's default headers, so
    /// every request carries it without per-call plumbing.
    pub(crate) fn post_url(&self, url: Url) -> RequestBuilder {
        self.inner.http.post(url)
    }

    /// Begins a `GET` request to a pre-built URL.
    pub(crate) fn get_url(&self, url: Url) -> RequestBuilder {
        self.inner.http.get(url)
    }

    /// Begins a `PUT` request to a pre-built URL.
    pub(crate) fn put_url(&self, url: Url) -> RequestBuilder {
        self.inner.http.put(url)
    }

    /// Sends a request, mapping any non-success status to [`Error::Service`].
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, err))]
    pub(crate) async fn send(&self, req: RequestBuilder) -> Result<ReqwestResponse> {
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
        timeout: Option<Duration>,
        max_retries: u32,
        mut headers: reqwest::header::HeaderMap,
    ) -> Result<Self> {
        // The bearer token is a constant credential for the life of the client, so it
        // lives in the default headers rather than being applied per request. This
        // also authenticates requests made through the `as_client*` escape hatches.
        if let Some(token) = token {
            let mut value: reqwest::header::HeaderValue = format!("Bearer {token}")
                .parse()
                .map_err(|e| Error::invalid_request("invalid bearer token", e))?;
            value.set_sensitive(true);
            headers.insert(reqwest::header::AUTHORIZATION, value);
        }

        let mut http = reqwest::Client::builder().default_headers(headers);
        if let Some(timeout) = timeout {
            http = http.timeout(timeout);
        }
        let inner = http.build()?;

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
            inner: Arc::new(ClientImpl { http, base_url }),
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
            c.route_url("summarize").unwrap().as_str(),
            "http://localhost:3000/summarize"
        );
        assert_eq!(
            c.route_url("/summarize").unwrap().as_str(),
            "http://localhost:3000/summarize"
        );
    }

    #[test]
    fn endpoint_preserves_a_path_prefix() {
        // A `/v1/` prefix must survive route joining (BentoML `path_prefix`).
        let c = client("http://localhost:3000/v1");
        assert_eq!(
            c.route_url("summarize").unwrap().as_str(),
            "http://localhost:3000/v1/summarize"
        );
    }
}
