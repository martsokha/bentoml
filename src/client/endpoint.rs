//! The [`Endpoint`] handle.

use std::borrow::Cow;

use bytes::Bytes;
use reqwest_middleware::RequestBuilder;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::client::multipart::Multipart;
use crate::client::{Client, EndpointResponse, Headers};
use crate::error::Result;

/// A handle to a single service endpoint, pairing a route with its [`Client`].
///
/// Obtain one with [`Client::endpoint`]. The route is named once; calls are made on
/// the handle rather than passing the route to each method. It carries the generic
/// [`call`], the body-specific `call_json` / `call_bytes` / `call_multipart`, async
/// task queues ([`submit`]), and (behind the `stream` feature) `stream`.
///
/// Per-call headers are attached with [`with_header`]: build a fresh handle per
/// request when they vary.
///
/// ```no_run
/// use bentoml::prelude::*;
/// use serde::{Deserialize, Serialize};
///
/// # #[derive(Serialize)] struct Req { text: String }
/// # #[derive(Deserialize)] struct Resp { summary: String }
/// # async fn run(client: Client) -> Result<()> {
/// let resp: Resp = client
///     .endpoint("summarize")
///     .with_request_id("req-42")
///     .call(&Req { text: "...".into() })
///     .await?;
/// # let _ = resp;
/// # Ok(())
/// # }
/// ```
///
/// [`call`]: Endpoint::call
/// [`submit`]: Endpoint::submit
/// [`with_header`]: Endpoint::with_header
#[derive(Debug, Clone)]
pub struct Endpoint {
    client: Client,
    route: Cow<'static, str>,
    headers: Headers,
}

impl Endpoint {
    pub(crate) fn new(client: Client, route: Cow<'static, str>) -> Self {
        Self {
            client,
            route,
            headers: Headers::default(),
        }
    }

    /// This endpoint's route.
    pub fn route(&self) -> &str {
        &self.route
    }

    /// The `x-request-id` set on this handle, if any. Used to enrich tracing spans.
    #[cfg(feature = "tracing")]
    pub(crate) fn request_id(&self) -> Option<&str> {
        self.headers.request_id()
    }

    /// Adds a header sent with every request made through this handle.
    ///
    /// Sent in addition to any configured on the client via
    /// [`ClientBuilder::with_header`], overriding those on a name clash. An invalid
    /// name or value surfaces as an error when the request is made.
    ///
    /// [`ClientBuilder::with_header`]: crate::ClientBuilder::with_header
    pub fn with_header(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Adds several headers sent with every request made through this handle.
    pub fn with_headers<K, V, I>(mut self, headers: I) -> Self
    where
        K: AsRef<str>,
        V: AsRef<str>,
        I: IntoIterator<Item = (K, V)>,
    {
        for (name, value) in headers {
            self.headers.insert(name, value);
        }
        self
    }

    /// Sets the `x-request-id` header for this request.
    ///
    /// Convenience for [`with_header`] with the `x-request-id` name, useful for
    /// correlating a single call in server logs and traces.
    ///
    /// [`with_header`]: Self::with_header
    pub fn with_request_id(self, id: impl AsRef<str>) -> Self {
        self.with_header("x-request-id", id)
    }

    /// Invokes the endpoint with the given JSON `payload`, returning the deserialized
    /// JSON response.
    ///
    /// This is the common JSON-in, JSON-out case, shorthand for
    /// `call_json(payload).await?.json().await`. For other response encodings, use
    /// [`call_json`] and read the [`EndpointResponse`] as you like. BentoML endpoints are
    /// `POST` by default.
    ///
    /// [`call_json`]: Self::call_json
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, payload), fields(route = %self.route, request_id = self.request_id()), err))]
    pub async fn call<T, R>(&self, payload: &T) -> Result<R>
    where
        T: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        self.call_json(payload).await?.json().await
    }

    /// Invokes the endpoint with the given JSON `payload`, returning the raw
    /// [`EndpointResponse`] to read as JSON, bytes, or text.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, payload), fields(route = %self.route, request_id = self.request_id()), err))]
    pub async fn call_json<T>(&self, payload: &T) -> Result<EndpointResponse>
    where
        T: Serialize + ?Sized,
    {
        let req = self.request(self.route())?.json(payload);
        Ok(EndpointResponse::new(self.client.send(req).await?))
    }

    /// Invokes the endpoint with a raw byte body, for endpoints that take a single
    /// positional binary ("root") input. Returns the raw [`EndpointResponse`].
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, body), fields(route = %self.route, request_id = self.request_id()), err))]
    pub async fn call_bytes(&self, body: impl Into<Bytes>) -> Result<EndpointResponse> {
        let req = self.request(self.route())?.body(body.into());
        Ok(EndpointResponse::new(self.client.send(req).await?))
    }

    /// Invokes the endpoint with a `multipart/form-data` body, for endpoints that
    /// take file or image inputs. Build the body with [`Multipart`]. Returns the raw
    /// [`EndpointResponse`].
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, body), fields(route = %self.route, request_id = self.request_id()), err))]
    pub async fn call_multipart(&self, body: Multipart) -> Result<EndpointResponse> {
        let req = self.request(self.route())?.multipart(body.into_form()?);
        Ok(EndpointResponse::new(self.client.send(req).await?))
    }

    /// The client this endpoint belongs to.
    pub(crate) fn client(&self) -> &Client {
        &self.client
    }

    /// Clones this endpoint's route for handing to a [`TaskHandle`].
    pub(crate) fn route_cow(&self) -> Cow<'static, str> {
        self.route.clone()
    }

    /// This endpoint's per-call headers, for propagating to a [`TaskHandle`].
    pub(crate) fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Begins a `POST` request to `route` with the bearer token and this endpoint's
    /// headers applied. The capability traits route through here so per-endpoint
    /// headers cover every operation.
    pub(crate) fn request(&self, route: &str) -> Result<RequestBuilder> {
        self.headers.apply(self.client.post(route)?)
    }
}
