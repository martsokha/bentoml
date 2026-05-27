//! Error and result types for the crate.

use std::time::Duration;

use crate::task::TaskStatus;

/// A convenient alias for results returned by this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// A boxed source error, used to preserve the cause chain.
type Source = Box<dyn std::error::Error + Send + Sync>;

/// The error type returned by client operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The request could not be constructed: an unparseable base URL or route, an
    /// invalid header, or a malformed multipart body.
    #[error("invalid request: {message}")]
    InvalidRequest {
        /// What was invalid.
        message: String,
        /// The underlying cause, if any.
        #[source]
        source: Option<Source>,
    },

    /// A response could not be decoded (e.g. invalid UTF-8 or malformed JSON in a
    /// streamed response).
    #[error("failed to decode response: {message}")]
    Decode {
        /// What failed to decode.
        message: String,
        /// The underlying cause, if any.
        #[source]
        source: Option<Source>,
    },

    /// The HTTP request failed: connection, timeout, retries exhausted, or the
    /// client could not be built.
    #[error("http transport error: {0}")]
    Transport(#[from] reqwest_middleware::Error),

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
    /// The HTTP status code, if this error carries one (a [`Service`] response, or a
    /// transport error generated from a response).
    ///
    /// [`Service`]: Error::Service
    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Service { status, .. } => Some(*status),
            Self::Transport(e) => e.status().map(|s| s.as_u16()),
            _ => None,
        }
    }

    /// An `InvalidRequest` error with a message and a source cause.
    pub(crate) fn invalid_request(message: impl Into<String>, source: impl Into<Source>) -> Self {
        Self::InvalidRequest {
            message: message.into(),
            source: Some(source.into()),
        }
    }

    /// An `InvalidRequest` error with just a message.
    pub(crate) fn invalid_message(message: impl Into<String>) -> Self {
        Self::InvalidRequest {
            message: message.into(),
            source: None,
        }
    }

    /// A `Decode` error with a message and a source cause.
    pub(crate) fn decode(message: impl Into<String>, source: impl Into<Source>) -> Self {
        Self::Decode {
            message: message.into(),
            source: Some(source.into()),
        }
    }

    /// A `Decode` error with just a message.
    pub(crate) fn decode_message(message: impl Into<String>) -> Self {
        Self::Decode {
            message: message.into(),
            source: None,
        }
    }

    /// Builds a `Service` error from a status code and raw response body, parsing
    /// BentoML's `{"error", "detail"}` envelope when the body is one.
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

/// Converts a URL parse error into an [`Error::InvalidRequest`].
impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Self::invalid_request("invalid url", e)
    }
}

/// Converts a client-build error into an [`Error::Transport`].
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Transport(e.into())
    }
}
