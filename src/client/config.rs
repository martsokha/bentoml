//! Client configuration.

use std::time::Duration;

/// The default request timeout applied when none is configured.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Resolved configuration for a [`Client`](crate::client::Client).
///
/// Most users construct this indirectly through
/// [`ClientBuilder`](crate::client::ClientBuilder) rather than by hand.
#[derive(Debug, Clone)]
pub struct Config {
    /// The base URL of the BentoML service, e.g. `http://localhost:3000`.
    pub base_url: String,
    /// An optional bearer token sent on each request.
    pub token: Option<String>,
    /// The per-request timeout.
    pub timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:3000".to_owned(),
            token: None,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}
