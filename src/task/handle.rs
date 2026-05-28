//! The [`TaskHandle`] returned by [`TaskEndpoint::submit`].
//!
//! [`TaskEndpoint::submit`]: crate::task::TaskEndpoint::submit

use std::borrow::Cow;
use std::future::Future;
use std::time::{Duration, Instant};

use bytes::Bytes;
use reqwest::Method;
use reqwest_middleware::RequestBuilder;
use serde::de::DeserializeOwned;

use super::join;
use super::model::{TaskInfo, TaskStatus};
use crate::client::{Client, Headers};
use crate::error::{Error, Result};

/// A handle to a submitted task, pairing its id with the client that created it.
///
/// Carries the per-call headers from the [`TaskEndpoint`] that submitted it, so they
/// apply to every status/result/retry/cancel request too.
///
/// [`TaskEndpoint`]: crate::task::TaskEndpoint
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

    /// Polls [`status`] until the task reaches a terminal state (completed, failed,
    /// or canceled), then returns that status.
    ///
    /// `sleep(interval)` is awaited between polls; the crate is runtime-agnostic, so
    /// the caller supplies the delay (with Tokio, pass `tokio::time::sleep`). Returns
    /// [`Error::Timeout`] if the task is still running after `timeout`. The returned
    /// status may be non-`Completed` — inspect it before reading the result.
    ///
    /// [`status`]: Self::status
    /// [`Error::Timeout`]: crate::Error::Timeout
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(skip(self, sleep), fields(route = %self.route, task_id = %self.task_id, request_id = self.request_id()), err)
    )]
    pub async fn wait<S, F>(
        &self,
        timeout: Duration,
        interval: Duration,
        mut sleep: S,
    ) -> Result<TaskStatus>
    where
        S: FnMut(Duration) -> F + Send,
        F: Future<Output = ()> + Send,
    {
        let deadline = Instant::now() + timeout;
        loop {
            let status = self.status().await?;
            if status.is_terminal() {
                return Ok(status);
            }
            if Instant::now() >= deadline {
                return Err(Error::Timeout { timeout });
            }
            sleep(interval).await;
        }
    }

    /// Fetches the completed result, deserialized as JSON into `R`.
    ///
    /// Checks the task status first and returns [`Error::TaskNotComplete`] unless the
    /// task has [`Completed`], so a pending, failed, or canceled task yields a clear
    /// error rather than a deserialization failure. Use [`bytes`] for a binary/file
    /// result or [`text`] for a text result.
    ///
    /// [`Error::TaskNotComplete`]: crate::Error::TaskNotComplete
    /// [`Completed`]: TaskStatus::Completed
    /// [`bytes`]: Self::bytes
    /// [`text`]: Self::text
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(skip(self), fields(route = %self.route, task_id = %self.task_id, request_id = self.request_id()), err)
    )]
    pub async fn json<R: DeserializeOwned>(&self) -> Result<R> {
        Ok(self.result_response().await?.json().await?)
    }

    /// Fetches the completed result as raw bytes, for a binary or file output.
    ///
    /// Like [`json`], this returns [`Error::TaskNotComplete`] unless the task has
    /// completed.
    ///
    /// [`json`]: Self::json
    /// [`Error::TaskNotComplete`]: crate::Error::TaskNotComplete
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(skip(self), fields(route = %self.route, task_id = %self.task_id, request_id = self.request_id()), err)
    )]
    pub async fn bytes(&self) -> Result<Bytes> {
        Ok(self.result_response().await?.bytes().await?)
    }

    /// Fetches the completed result as UTF-8 text, for a `text/plain` output.
    ///
    /// Like [`json`], this returns [`Error::TaskNotComplete`] unless the task has
    /// completed.
    ///
    /// [`json`]: Self::json
    /// [`Error::TaskNotComplete`]: crate::Error::TaskNotComplete
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(skip(self), fields(route = %self.route, task_id = %self.task_id, request_id = self.request_id()), err)
    )]
    pub async fn text(&self) -> Result<String> {
        Ok(self.result_response().await?.text().await?)
    }

    /// Checks the task is [`Completed`], then fetches the raw result response. Shared
    /// by `json` / `bytes` / `text`.
    ///
    /// [`Completed`]: TaskStatus::Completed
    async fn result_response(&self) -> Result<reqwest::Response> {
        let status = self.status().await?;
        if status != TaskStatus::Completed {
            return Err(Error::TaskNotComplete {
                task_id: self.task_id.clone(),
                status,
            });
        }
        let req = self.request("get", Method::GET)?;
        self.client.send(req).await
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
