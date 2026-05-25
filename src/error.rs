//! Error and result types for the crate.

use std::time::Duration;

use crate::model::TaskStatus;

/// A convenient alias for results returned by this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The error type returned by client operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A URL could not be parsed: either the configured base URL, or a route joined
    /// onto it.
    #[error("invalid url: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// A custom header name or value was invalid.
    #[error("invalid header: {0}")]
    InvalidHeader(String),

    /// A streamed response chunk could not be decoded (e.g. invalid UTF-8).
    #[error("failed to decode response: {0}")]
    Decode(String),

    /// An error occurred while performing the HTTP request.
    #[error("http transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// An error occurred in the request middleware stack (e.g. retries).
    #[error("http middleware error: {0}")]
    Middleware(#[from] reqwest_middleware::Error),

    /// The service did not become ready within the configured timeout.
    #[error("timed out after {timeout:?} waiting for the service")]
    Timeout {
        /// The timeout that elapsed.
        timeout: Duration,
    },

    /// The service responded with a non-success status code.
    ///
    /// `message` is the `error` field of BentoML's JSON error envelope when present,
    /// otherwise the raw response body. `detail` carries the `detail` field, present
    /// on validation errors.
    #[error("service returned status {status}: {message}")]
    Service {
        /// The HTTP status code returned by the service.
        status: u16,
        /// The error message from the envelope, or the raw body.
        message: String,
        /// Structured error detail, when the service provides it (e.g. validation
        /// errors), as raw JSON.
        detail: Option<serde_json::Value>,
    },

    /// A task result was requested but the task is not in a terminal `Completed`
    /// state.
    #[error("task {task_id} is not complete: status is {status:?}")]
    TaskNotComplete {
        /// The task identifier.
        task_id: String,
        /// The task's current status.
        status: TaskStatus,
    },
}

/// BentoML's JSON error envelope: `{"error": "...", "detail": ...}`.
#[derive(serde::Deserialize)]
struct ErrorEnvelope {
    error: String,
    #[serde(default)]
    detail: Option<serde_json::Value>,
}

impl Error {
    /// Builds a [`Error::Service`] from a status code and raw response body,
    /// parsing BentoML's `{"error", "detail"}` envelope when the body is one.
    pub(crate) fn service(status: u16, body: &str) -> Self {
        match serde_json::from_str::<ErrorEnvelope>(body) {
            Ok(env) => Self::Service {
                status,
                message: env.error,
                detail: env.detail,
            },
            // Not an envelope (HTML, empty, plain text): keep the raw body.
            Err(_) => Self::Service {
                status,
                message: body.to_owned(),
                detail: None,
            },
        }
    }
}
