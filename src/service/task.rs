//! Async task-queue endpoints (`@bentoml.task`).
//!
//! A task endpoint named `generate` is served at five routes derived from its base
//! route: `POST generate/submit`, `GET generate/status`, `GET generate/get`,
//! `POST generate/retry`, and `PUT generate/cancel`. The latter four identify the
//! task with a `task_id` query parameter.

use std::future::Future;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::client::Client;
use crate::error::{Error, Result};
use crate::model::{TaskInfo, TaskStatus};

/// A handle to a submitted task, pairing its id with the client that created it.
#[derive(Debug, Clone)]
pub struct TaskHandle {
    client: Client,
    route: String,
    task_id: String,
}

impl TaskHandle {
    /// The unique identifier of the task.
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    /// Fetches the current status of the task.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(skip(self), fields(route = %self.route, task_id = %self.task_id), err)
    )]
    pub async fn status(&self) -> Result<TaskStatus> {
        Ok(self
            .client
            .get_status(&self.route, &self.task_id)
            .await?
            .status)
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
        tracing::instrument(skip(self), fields(route = %self.route, task_id = %self.task_id), err)
    )]
    pub async fn get<R: DeserializeOwned>(&self) -> Result<R> {
        let status = self.status().await?;
        if status != TaskStatus::Completed {
            return Err(Error::TaskNotComplete {
                task_id: self.task_id.clone(),
                status,
            });
        }
        self.client.get_result(&self.route, &self.task_id).await
    }

    /// Re-runs the task, returning a handle to the new run.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(skip(self), fields(route = %self.route, task_id = %self.task_id), err)
    )]
    pub async fn retry(&self) -> Result<TaskHandle> {
        self.client.retry(&self.route, &self.task_id).await
    }

    /// Cancels the task before it starts executing.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(skip(self), fields(route = %self.route, task_id = %self.task_id), err)
    )]
    pub async fn cancel(&self) -> Result<()> {
        self.client.cancel(&self.route, &self.task_id).await
    }
}

/// Async task-queue operations against a BentoML service.
///
/// Implemented for [`Client`]. The high-level [`submit`] returns a [`TaskHandle`]
/// that wraps the lower-level id-based methods.
///
/// [`submit`]: Tasks::submit
pub trait Tasks {
    /// Submits a task to `route`, returning a [`TaskHandle`] for tracking it.
    fn submit<T>(
        &self,
        route: &str,
        payload: &T,
    ) -> impl Future<Output = Result<TaskHandle>> + Send
    where
        T: Serialize + ?Sized + Sync;

    /// Fetches the status of the task `task_id` on `route`.
    fn get_status(
        &self,
        route: &str,
        task_id: &str,
    ) -> impl Future<Output = Result<TaskInfo>> + Send;

    /// Fetches the result of the task `task_id` on `route`, deserialized as `R`.
    fn get_result<R: DeserializeOwned>(
        &self,
        route: &str,
        task_id: &str,
    ) -> impl Future<Output = Result<R>> + Send;

    /// Re-runs the task `task_id` on `route`, returning a handle to the new run.
    fn retry(&self, route: &str, task_id: &str) -> impl Future<Output = Result<TaskHandle>> + Send;

    /// Cancels the task `task_id` on `route` before it starts executing.
    fn cancel(&self, route: &str, task_id: &str) -> impl Future<Output = Result<()>> + Send;
}

impl Tasks for Client {
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, payload), err))]
    async fn submit<T>(&self, route: &str, payload: &T) -> Result<TaskHandle>
    where
        T: Serialize + ?Sized + Sync,
    {
        let route = route.trim_start_matches('/').to_owned();
        let req = self.post(&join(&route, "submit"))?.json(payload);
        let info: TaskInfo = self.send(req).await?.json().await?;
        Ok(TaskHandle {
            client: self.clone(),
            route,
            task_id: info.task_id,
        })
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self), err))]
    async fn get_status(&self, route: &str, task_id: &str) -> Result<TaskInfo> {
        let url = self.endpoint_query(&join(route, "status"), "task_id", task_id)?;
        Ok(self.send(self.get_url(url)).await?.json().await?)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self), err))]
    async fn get_result<R: DeserializeOwned>(&self, route: &str, task_id: &str) -> Result<R> {
        let url = self.endpoint_query(&join(route, "get"), "task_id", task_id)?;
        Ok(self.send(self.get_url(url)).await?.json().await?)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self), err))]
    async fn retry(&self, route: &str, task_id: &str) -> Result<TaskHandle> {
        let route = route.trim_start_matches('/').to_owned();
        let url = self.endpoint_query(&join(&route, "retry"), "task_id", task_id)?;
        let info: TaskInfo = self.send(self.post_url(url)).await?.json().await?;
        Ok(TaskHandle {
            client: self.clone(),
            route,
            task_id: info.task_id,
        })
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self), err))]
    async fn cancel(&self, route: &str, task_id: &str) -> Result<()> {
        let url = self.endpoint_query(&join(route, "cancel"), "task_id", task_id)?;
        self.send(self.put_url(url)).await?;
        Ok(())
    }
}

/// Joins a base route and a task operation suffix into `route/op`.
fn join(route: &str, op: &str) -> String {
    format!("{}/{op}", route.trim_end_matches('/'))
}
