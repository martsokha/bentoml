//! Shared data models for BentoML services.
//!
//! Endpoint request and response shapes are service-specific and defined by the
//! caller. This module holds types common to the BentoML protocol itself, such as
//! health-check and metadata responses.

mod health;
mod task;

pub use self::health::HealthStatus;
pub use self::task::{TaskInfo, TaskStatus};
