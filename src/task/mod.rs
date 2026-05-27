//! Async task-queue endpoints (`@bentoml.task`).
//!
//! A task endpoint named `generate` is served at five routes derived from its base
//! route: `POST generate/submit`, `GET generate/status`, `GET generate/get`,
//! `POST generate/retry`, and `PUT generate/cancel`. The latter four identify the
//! task with a `task_id` query parameter.

mod handle;
mod model;

use serde::Serialize;

pub use self::handle::TaskHandle;
pub use self::model::{TaskInfo, TaskStatus};
use crate::client::Endpoint;
use crate::error::Result;

impl Endpoint {
    /// Submits the endpoint as a task (`@bentoml.task`), returning a [`TaskHandle`]
    /// for tracking it.
    ///
    /// The handle carries the id-based operations (`status`, `get`, `retry`,
    /// `cancel`).
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, payload), fields(route = %self.route()), err))]
    pub async fn submit<T>(&self, payload: &T) -> Result<TaskHandle>
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
