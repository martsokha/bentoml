//! Async task-queue models.

use jiff::Timestamp;
use jiff::civil::DateTime;
use serde::{Deserialize, Serialize};

/// The lifecycle state of a submitted task.
///
/// Each variant is pinned to the exact wire value used by BentoML's `ResultStatus`,
/// rather than derived from the variant name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TaskStatus {
    /// Queued, not yet started.
    #[serde(rename = "pending")]
    Pending,
    /// Currently executing.
    #[serde(rename = "in_progress")]
    InProgress,
    /// Finished successfully; the result is available via [`TaskHandle::get`].
    ///
    /// [`TaskHandle::get`]: crate::service::TaskHandle::get
    #[serde(rename = "completed")]
    Completed,
    /// Finished with an error.
    #[serde(rename = "failed")]
    Failed,
    /// Canceled before execution.
    #[serde(rename = "canceled")]
    Canceled,
}

impl TaskStatus {
    /// Whether the task has reached a terminal state (completed, failed, canceled).
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Canceled)
    }
}

/// The status payload returned by the `submit` and `status` task endpoints.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct TaskInfo {
    /// The unique identifier of the task.
    pub task_id: String,
    /// The current status of the task.
    pub status: TaskStatus,
    /// When the task was submitted.
    ///
    /// Absent from the `submit` response, present from `status`. BentoML reports
    /// this as a naive (offset-less) UTC datetime.
    #[serde(default)]
    pub created_at: Option<DateTime>,
    /// When the task began executing, if it has started.
    ///
    /// BentoML reports this as a naive (offset-less) UTC datetime.
    #[serde(default)]
    pub executed_at: Option<DateTime>,
    /// When the task finished, if it has completed.
    ///
    /// Unlike the other two, BentoML reports this as a timezone-aware UTC instant,
    /// hence [`Timestamp`] rather than [`DateTime`].
    #[serde(default)]
    pub completed_at: Option<Timestamp>,
}
