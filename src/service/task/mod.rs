//! Async task-queue endpoints (`@bentoml.task`).
//!
//! A task endpoint named `generate` is served at five routes derived from its base
//! route: `POST generate/submit`, `GET generate/status`, `GET generate/get`,
//! `POST generate/retry`, and `PUT generate/cancel`. The latter four identify the
//! task with a `task_id` query parameter.

mod handle;

use std::future::Future;

use serde::Serialize;

pub use self::handle::TaskHandle;
use crate::client::Endpoint;
use crate::error::Result;
use crate::model::TaskInfo;

/// Async task-queue operations against a BentoML service.
///
/// Implemented for [`Endpoint`]. [`submit`] returns a [`TaskHandle`] that carries the
/// id-based operations (`status`, `get`, `retry`, `cancel`).
///
/// [`submit`]: Tasks::submit
pub trait Tasks {
    /// Submits the endpoint as a task, returning a [`TaskHandle`] for tracking it.
    fn submit<T>(&self, payload: &T) -> impl Future<Output = Result<TaskHandle>> + Send
    where
        T: Serialize + ?Sized + Sync;
}

impl Tasks for Endpoint {
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, payload), fields(route = %self.route()), err))]
    async fn submit<T>(&self, payload: &T) -> Result<TaskHandle>
    where
        T: Serialize + ?Sized + Sync,
    {
        let req = self.request(&join(self.route(), "submit"))?.json(payload);
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
