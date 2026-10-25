//! The [`Endpoint`] handle and the shared [`EndpointBase`] behind it and
//! [`TaskEndpoint`].
//!
//! [`TaskEndpoint`]: crate::task::TaskEndpoint

use std::borrow::Cow;

use reqwest_middleware::RequestBuilder;
use serde::de::DeserializeOwned;

use crate::client::{Client, EndpointReply, Headers};
use crate::error::Result;
use crate::input::Input;

/// State and plumbing shared by [`Endpoint`] and [`TaskEndpoint`]: the route, the
/// client, the per-call headers, and the request builder both kinds route through.
///
/// [`TaskEndpoint`]: crate::task::TaskEndpoint
#[derive(Debug, Clone)]
pub(crate) struct EndpointBase {
    client: Client,
    route: Cow<'static, str>,
    headers: Headers,
}

impl EndpointBase {
    pub(crate) fn new(client: Client, route: Cow<'static, str>) -> Self {
        Self {
            client,
            route,
            headers: Headers::default(),
        }
    }

    /// This endpoint's route.
    pub(crate) fn route(&self) -> &str {
        &self.route
    }

    /// The `x-request-id` set on this handle, if any. Used to enrich tracing spans.
    #[cfg(feature = "tracing")]
    pub(crate) fn request_id(&self) -> Option<&str> {
        self.headers.request_id()
    }

    /// Records a header sent with every request made through this handle.
    pub(crate) fn insert_header(&mut self, name: impl AsRef<str>, value: impl AsRef<str>) {
        self.headers.insert(name, value);
    }

    /// The client this endpoint belongs to.
    pub(crate) fn client(&self) -> &Client {
        &self.client
    }

    /// Clones this endpoint's route for handing to a [`TaskHandle`].
    ///
    /// [`TaskHandle`]: crate::task::TaskHandle
    pub(crate) fn route_cow(&self) -> Cow<'static, str> {
        self.route.clone()
    }

    /// This endpoint's per-call headers, for propagating to a [`TaskHandle`].
    ///
    /// [`TaskHandle`]: crate::task::TaskHandle
    pub(crate) fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Begins a `POST` request to `route` with the bearer token and this endpoint's
    /// headers applied. The kind-specific request methods route through here so
    /// per-endpoint headers cover every operation.
    pub(crate) fn request(&self, route: &str) -> Result<RequestBuilder> {
        self.headers.apply(self.client.post(route)?)
    }
}

/// A handle to a synchronous service endpoint (`@bentoml.api`), pairing a route with
/// its [`Client`].
///
/// Obtain one with [`Client::endpoint`]. The route is named once; calls are made on
/// the handle rather than passing the route to each method. [`call`] sends an
/// [`Input`] and returns the raw [`EndpointReply`]; for the common JSON-out case
/// [`invoke`] deserializes in one step. Async task queues (`@bentoml.task`) use
/// [`TaskEndpoint`] from [`Client::task`] instead.
///
/// The [`Input`] follows the handler's signature: a reference to a struct mirroring
/// its parameters converts directly, and [`Input::named`], [`Input::root`],
/// [`Input::bytes`], and [`Input::multipart`] cover the other shapes.
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
///     .invoke(&Req { text: "...".into() })
///     .await?;
/// # let _ = resp;
/// # Ok(())
/// # }
/// ```
///
/// [`call`]: Endpoint::call
/// [`invoke`]: Endpoint::invoke
/// [`with_header`]: Endpoint::with_header
/// [`TaskEndpoint`]: crate::task::TaskEndpoint
#[derive(Debug, Clone)]
pub struct Endpoint {
    base: EndpointBase,
}

impl Endpoint {
    pub(crate) fn new(client: Client, route: Cow<'static, str>) -> Self {
        Self {
            base: EndpointBase::new(client, route),
        }
    }

    /// This endpoint's route.
    pub fn route(&self) -> &str {
        self.base.route()
    }

    /// Adds a header sent with every request made through this handle.
    ///
    /// Sent in addition to any configured on the client via
    /// [`ClientBuilder::with_header`], overriding those on a name clash. An invalid
    /// name or value surfaces as an error when the request is made.
    ///
    /// [`ClientBuilder::with_header`]: crate::ClientBuilder::with_header
    pub fn with_header(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.base.insert_header(name, value);
        self
    }

    /// Adds several headers sent with every request made through this handle.
    pub fn with_headers<N, V, I>(mut self, headers: I) -> Self
    where
        N: AsRef<str>,
        V: AsRef<str>,
        I: IntoIterator<Item = (N, V)>,
    {
        for (name, value) in headers {
            self.base.insert_header(name, value);
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

    /// Invokes the endpoint with `input`, returning the raw [`EndpointReply`] to
    /// read as `.json::<R>()`, `.bytes()`, `.text()`, or (feature `stream`)
    /// `.stream()`.
    ///
    /// BentoML endpoints are `POST` by default. For the common JSON-out case,
    /// [`invoke`] deserializes the response in one step.
    ///
    /// [`invoke`]: Self::invoke
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(route = %self.base.route(), request_id = self.base.request_id()), err))]
    pub async fn call(&self, input: impl Into<Input>) -> Result<EndpointReply> {
        let req = input.into().attach(self.base.request(self.base.route())?)?;
        Ok(EndpointReply::new(self.base.client().send(req).await?))
    }

    /// Invokes the endpoint with `input` and deserializes the JSON response into `R`.
    ///
    /// Shorthand for `call(input).await?.json().await`. Use [`call`] when you need
    /// the response as bytes, text, or a stream.
    ///
    /// ```no_run
    /// use bentoml::prelude::*;
    /// use serde::{Deserialize, Serialize};
    ///
    /// # #[derive(Serialize)] struct NerRequest { text: String }
    /// # #[derive(Deserialize)] struct NerResponse { entities: Vec<String> }
    /// # async fn run(client: Client, requests: Vec<NerRequest>) -> Result<()> {
    /// // def recognize(self, requests: list[NerRequest]) -> list[NerResponse]
    /// let resp: Vec<NerResponse> = client
    ///     .endpoint("recognize")
    ///     .invoke(Input::named("requests", &requests))
    ///     .await?;
    /// # let _ = resp;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`call`]: Self::call
    pub async fn invoke<R>(&self, input: impl Into<Input>) -> Result<R>
    where
        R: DeserializeOwned,
    {
        self.call(input).await?.json().await
    }
}
