//! Async task-queue endpoints (`@bentoml.task`).
//!
//! A task endpoint named `generate` is served at five routes derived from its base
//! route: `POST generate/submit`, `GET generate/status`, `GET generate/get`,
//! `POST generate/retry`, and `PUT generate/cancel`. The latter four identify the
//! task with a `task_id` query parameter.

mod handle;
mod model;

use bytes::Bytes;
use reqwest_middleware::RequestBuilder;
use serde::Serialize;

pub use self::handle::TaskHandle;
pub use self::model::{TaskInfo, TaskStatus};
use crate::client::Endpoint;
use crate::client::multipart::Multipart;
use crate::error::Result;

impl Endpoint {
    /// Submits the endpoint as a task (`@bentoml.task`) with a JSON `payload`,
    /// returning a [`TaskHandle`] for tracking it.
    ///
    /// The handle carries the id-based operations (`status`, result readers, `retry`,
    /// `cancel`). Use [`submit_bytes`] or [`submit_multipart`] for non-JSON task
    /// inputs.
    ///
    /// [`submit_bytes`]: Self::submit_bytes
    /// [`submit_multipart`]: Self::submit_multipart
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, payload), fields(route = %self.route(), request_id = self.request_id()), err))]
    pub async fn submit<T>(&self, payload: &T) -> Result<TaskHandle>
    where
        T: Serialize + ?Sized + Sync,
    {
        self.submit_with(self.submit_request()?.json(payload)).await
    }

    /// Submits the endpoint as a task with a raw byte body, for task endpoints that
    /// take a single positional binary ("root") input.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, body), fields(route = %self.route(), request_id = self.request_id()), err))]
    pub async fn submit_bytes(&self, body: impl Into<Bytes>) -> Result<TaskHandle> {
        self.submit_with(self.submit_request()?.body(body.into()))
            .await
    }

    /// Submits the endpoint as a task with a `multipart/form-data` body, for task
    /// endpoints that take file or image inputs. Build the body with [`Multipart`].
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, body), fields(route = %self.route(), request_id = self.request_id()), err))]
    pub async fn submit_multipart(&self, body: Multipart) -> Result<TaskHandle> {
        self.submit_with(self.submit_request()?.multipart(body.into_form()?))
            .await
    }

    /// Begins the `POST {route}/submit` request, with bearer token and headers.
    fn submit_request(&self) -> Result<RequestBuilder> {
        self.request(&join(self.route(), "submit"))
    }

    /// Sends a prepared submit request and builds the resulting [`TaskHandle`].
    async fn submit_with(&self, req: RequestBuilder) -> Result<TaskHandle> {
        let info: TaskInfo = self.client().send(req).await?.json().await?;
        Ok(TaskHandle::new(
            self.client().clone(),
            self.route_cow(),
            info.task_id,
            self.headers().clone(),
        ))
    }
}

/// Joins a base route and a task operation suffix into `route/op`.
pub(super) fn join(route: &str, op: &str) -> String {
    format!("{}/{op}", route.trim_end_matches('/'))
}
