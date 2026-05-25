//! Service-level abstractions over the raw [`Client`](crate::client::Client).
//!
//! The client exposes a generic [`call`](crate::client::Client::call) for invoking
//! arbitrary endpoints. This module is the home for higher-level, ergonomic wrappers
//! built on top of it — for example, a typed facade over a specific service, or
//! helpers for streaming endpoints. It is intentionally minimal for now.
