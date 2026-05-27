//! The [`TaskHandle`] returned by [`Endpoint::submit`].
//!
//! [`Endpoint::submit`]: crate::Endpoint::submit

use std::borrow::Cow;

use reqwest::Method;
use reqwest_middleware::RequestBuilder;
use serde::de::DeserializeOwned;

use super::join;
use super::model::{TaskInfo, TaskStatus};
use crate::client::{Client, Headers};
use crate::error::{Error, Result};

/// A handle to a submitted task, pairing its id with the client that created it.
///
/// Carries the per-call headers from the [`Endpoint`] that submitted it, so they
/// apply to every status/result/retry/cancel request too.
///
/// [`Endpoint`]: crate::Endpoint
#[derive(Debug, Clone)]
pub struct TaskHandle {
    client: Client,
    route: Cow<'static, str>,
    task_id: String,
    headers: Headers,
}

impl TaskHandle {
    pub(super) fn new(
        client: Client,
        route: Cow<'static, str>,
        task_id: String,
        headers: Headers,
    ) -> Self {
        Self {
            client,
            route,
            task_id,
            headers,
        }
    }

    /// The unique identifier of the task.
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    /// The `x-request-id` carried from the submitting endpoint, for span correlation.
    #[cfg(feature = "tracing")]
    fn request_id(&self) -> Option<&str> {
        self.headers.request_id()
    }

    /// Builds a `task_id`-scoped request for `op`, with bearer token and headers.
    fn request(&self, op: &str, method: Method) -> Result<RequestBuilder> {
        let url = self
            .client
            .endpoint_query(&join(&self.route, op), "task_id", &self.task_id)?;
        let req = match method {
            Method::GET => self.client.get_url(url),
            Method::PUT => self.client.put_url(url),
            _ => self.client.post_url(url),
        };
        self.headers.apply(req)
    }

    /// Fetches the current status of the task.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(skip(self), fields(route = %self.route, task_id = %self.task_id, request_id = self.request_id()), err)
    )]
    pub async fn status(&self) -> Result<TaskStatus> {
        let req = self.request("status", Method::GET)?;
        let info: TaskInfo = self.client.send(req).await?.json().await?;
        Ok(info.status)
    }

    /// Fetches the completed result of the task, deserialized as `R`.
    ///
    /// Checks the task status first and returns [`Error::TaskNotComplete`] unless the
    /// task has [`Completed`], so a pending, failed, or canceled task yields a clear
    /// error rather than a deserialization failure.
    ///
    /// [`Error::TaskNotComplete`]: crate::Error::TaskNotComplete
    /// [`Completed`]: TaskStatus::Completed
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(skip(self), fields(route = %self.route, task_id = %self.task_id, request_id = self.request_id()), err)
    )]
    pub async fn get<R: DeserializeOwned>(&self) -> Result<R> {
        let status = self.status().await?;
        if status != TaskStatus::Completed {
            return Err(Error::TaskNotComplete {
                task_id: self.task_id.clone(),
                status,
            });
        }
        let req = self.request("get", Method::GET)?;
        Ok(self.client.send(req).await?.json().await?)
    }

    /// Re-runs the task, returning a handle to the new run.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(skip(self), fields(route = %self.route, task_id = %self.task_id, request_id = self.request_id()), err)
    )]
    pub async fn retry(&self) -> Result<TaskHandle> {
        let req = self.request("retry", Method::POST)?;
        let info: TaskInfo = self.client.send(req).await?.json().await?;
        Ok(TaskHandle {
            client: self.client.clone(),
            route: self.route.clone(),
            task_id: info.task_id,
            headers: self.headers.clone(),
        })
    }

    /// Cancels the task before it starts executing.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(skip(self), fields(route = %self.route, task_id = %self.task_id, request_id = self.request_id()), err)
    )]
    pub async fn cancel(&self) -> Result<()> {
        let req = self.request("cancel", Method::PUT)?;
        self.client.send(req).await?;
        Ok(())
    }
}
