//! Async task-queue models.

use serde::{Deserialize, Serialize};

/// The lifecycle state of a submitted task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TaskStatus {
    /// Queued, not yet started.
    Pending,
    /// Currently executing.
    Running,
    /// Finished successfully; the result is available via [`Tasks::get_result`].
    ///
    /// [`Tasks::get_result`]: crate::service::Tasks::get_result
    Success,
    /// Finished with an error.
    Failure,
    /// Cancelled before execution.
    Cancelled,
}

impl TaskStatus {
    /// Whether the task has reached a terminal state (success, failure, cancelled).
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Success | Self::Failure | Self::Cancelled)
    }
}

/// The status payload returned by the `status` and `submit` task endpoints.
#[derive(Debug, Clone, Deserialize)]
pub struct TaskInfo {
    /// The unique identifier of the task.
    pub task_id: String,
    /// The current status of the task.
    pub status: TaskStatus,
}
