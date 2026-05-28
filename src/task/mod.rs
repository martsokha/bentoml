//! Async task-queue endpoints (`@bentoml.task`).
//!
//! A task endpoint named `generate` is served at five routes derived from its base
//! route: `POST generate/submit`, `GET generate/status`, `GET generate/get`,
//! `POST generate/retry`, and `PUT generate/cancel`. The latter four identify the
//! task with a `task_id` query parameter.
//!
//! Obtain a [`TaskEndpoint`] with [`Client::task`], submit to get a [`TaskHandle`],
//! then track it with `status` / `wait` / the result readers.
//!
//! [`Client::task`]: crate::Client::task

mod endpoint;
mod handle;
mod model;

pub use self::endpoint::TaskEndpoint;
pub use self::handle::TaskHandle;
pub use self::model::{TaskInfo, TaskStatus};

/// Joins a base route and a task operation suffix into `route/op`.
pub(super) fn join(route: &str, op: &str) -> String {
    format!("{}/{op}", route.trim_end_matches('/'))
}
