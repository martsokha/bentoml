//! Error and result types for the crate.

use std::fmt;

/// A convenient alias for results returned by this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The error type returned by client operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The configured base URL could not be parsed.
    #[error("invalid base url: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),

    /// The client was misconfigured and could not be built.
    #[error("client configuration error: {0}")]
    Builder(String),

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
        timeout: std::time::Duration,
    },

    /// The service responded with a non-success status code.
    #[error("service returned status {status}: {message}")]
    Service {
        /// The HTTP status code returned by the service.
        status: u16,
        /// The response body, if any, as returned by the service.
        message: String,
    },

    /// A request or response body could not be (de)serialized.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl Error {
    /// Constructs a [`Error::Service`] from a status code and message.
    pub(crate) fn service(status: u16, message: impl fmt::Display) -> Self {
        Self::Service {
            status,
            message: message.to_string(),
        }
    }
}
