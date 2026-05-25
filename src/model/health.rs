//! Health-check models.

use serde::{Deserialize, Serialize};

/// The readiness/liveness status of a BentoML service.
///
/// BentoML exposes `/readyz` and `/livez` endpoints that signal status through the
/// HTTP status code rather than a body, so this type is primarily a convenience for
/// callers that want to model the distinction explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// The service is ready to serve requests.
    Ready,
    /// The service is alive but not yet ready.
    Live,
    /// The service is not healthy.
    Unhealthy,
}
