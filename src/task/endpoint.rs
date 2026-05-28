//! The [`TaskEndpoint`] handle for async task queues (`@bentoml.task`).

use std::borrow::Cow;

use bytes::Bytes;
use reqwest_middleware::RequestBuilder;
use serde::Serialize;

use super::handle::TaskHandle;
use super::join;
use super::model::TaskInfo;
use crate::client::multipart::Multipart;
use crate::client::{Client, EndpointBase};
use crate::error::Result;

/// A handle to an async task endpoint (`@bentoml.task`), pairing a route with its
/// [`Client`].
///
/// Obtain one with [`Client::task`]. Submitting returns a [`TaskHandle`] that carries
/// the id-based operations (`status`, `wait`, result readers, `retry`, `cancel`). The
/// body-specific [`submit`] / `submit_bytes` / `submit_multipart` mirror the `call`
/// family on a synchronous [`Endpoint`]; only the task surface is available here.
///
/// Per-call headers are attached with [`with_header`] and propagate to the
/// [`TaskHandle`], covering its status/result/retry/cancel requests too.
///
/// ```no_run
/// use bentoml::prelude::*;
/// use serde::Serialize;
///
/// # #[derive(Serialize)] struct Req { prompt: String }
/// # async fn run(client: Client) -> Result<()> {
/// let task = client
///     .task("generate")
///     .submit(&Req { prompt: "...".into() })
///     .await?;
/// # let _ = task;
/// # Ok(())
/// # }
/// ```
///
/// [`Client::task`]: crate::Client::task
/// [`submit`]: Self::submit
/// [`with_header`]: Self::with_header
/// [`Endpoint`]: crate::Endpoint
#[derive(Debug, Clone)]
pub struct TaskEndpoint {
    base: EndpointBase,
}

impl TaskEndpoint {
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

    /// Submits the endpoint as a task with a JSON `payload`, returning a [`TaskHandle`]
    /// for tracking it.
    ///
    /// The handle carries the id-based operations (`status`, `wait`, result readers,
    /// `retry`, `cancel`). Use [`submit_bytes`] or [`submit_multipart`] for non-JSON
    /// task inputs.
    ///
    /// [`submit_bytes`]: Self::submit_bytes
    /// [`submit_multipart`]: Self::submit_multipart
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, payload), fields(route = %self.base.route(), request_id = self.base.request_id()), err))]
    pub async fn submit<T>(&self, payload: &T) -> Result<TaskHandle>
    where
        T: Serialize + ?Sized + Sync,
    {
        self.submit_with(self.submit_request()?.json(payload)).await
    }

    /// Submits the endpoint as a task with a raw byte body, for task endpoints that
    /// take a single positional binary ("root") input.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, body), fields(route = %self.base.route(), request_id = self.base.request_id()), err))]
    pub async fn submit_bytes(&self, body: impl Into<Bytes>) -> Result<TaskHandle> {
        self.submit_with(self.submit_request()?.body(body.into()))
            .await
    }

    /// Submits the endpoint as a task with a `multipart/form-data` body, for task
    /// endpoints that take file or image inputs. Build the body with [`Multipart`].
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, body), fields(route = %self.base.route(), request_id = self.base.request_id()), err))]
    pub async fn submit_multipart(&self, body: Multipart) -> Result<TaskHandle> {
        self.submit_with(self.submit_request()?.multipart(body.into_form()?))
            .await
    }

    /// Begins the `POST {route}/submit` request, with bearer token and headers.
    fn submit_request(&self) -> Result<RequestBuilder> {
        self.base.request(&join(self.base.route(), "submit"))
    }

    /// Sends a prepared submit request and builds the resulting [`TaskHandle`].
    async fn submit_with(&self, req: RequestBuilder) -> Result<TaskHandle> {
        let info: TaskInfo = self.base.client().send(req).await?.json().await?;
        Ok(TaskHandle::new(
            self.base.client().clone(),
            self.base.route_cow(),
            info.task_id,
            self.base.headers().clone(),
        ))
    }
}
