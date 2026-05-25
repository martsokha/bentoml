//! Shared data models for BentoML services.
//!
//! Endpoint request and response shapes are service-specific and defined by the
//! caller. This module holds types common to the BentoML protocol itself, such as
//! the async task-queue status models.

mod task;

pub use self::task::{TaskInfo, TaskStatus};
