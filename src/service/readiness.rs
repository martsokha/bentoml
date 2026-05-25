//! Health-check and readiness helpers.

use std::future::Future;
use std::time::{Duration, Instant};

use crate::client::Client;
use crate::error::{Error, Result};

/// Health-check operations against a BentoML service.
///
/// BentoML services expose `/readyz` and `/livez` endpoints that report status
/// through the HTTP status code. This trait is implemented for [`Client`].
pub trait Readiness {
    /// Returns whether the service reports itself ready, via `/readyz`.
    fn is_ready(&self) -> impl Future<Output = Result<bool>> + Send;

    /// Returns whether the service reports itself alive, via `/livez`.
    fn is_live(&self) -> impl Future<Output = Result<bool>> + Send;

    /// Polls [`is_ready`](Readiness::is_ready) until it returns `true` or `timeout`
    /// elapses, awaiting `sleep(interval)` between attempts.
    ///
    /// The crate is runtime-agnostic, so the caller supplies the delay: `sleep` is
    /// invoked with `interval` and the returned future is awaited. With Tokio, pass
    /// `tokio::time::sleep`.
    ///
    /// Returns [`Error::Timeout`] if the service does not become ready in time.
    fn wait_until_ready<S, F>(
        &self,
        timeout: Duration,
        interval: Duration,
        sleep: S,
    ) -> impl Future<Output = Result<()>> + Send
    where
        S: FnMut(Duration) -> F + Send,
        F: Future<Output = ()> + Send;
}

impl Readiness for Client {
    async fn is_ready(&self) -> Result<bool> {
        self.health("readyz").await
    }

    async fn is_live(&self) -> Result<bool> {
        self.health("livez").await
    }

    async fn wait_until_ready<S, F>(
        &self,
        timeout: Duration,
        interval: Duration,
        mut sleep: S,
    ) -> Result<()>
    where
        S: FnMut(Duration) -> F + Send,
        F: Future<Output = ()> + Send,
    {
        let deadline = Instant::now() + timeout;
        loop {
            // Ignore transient errors (e.g. connection refused during startup);
            // only the deadline ends the loop.
            if matches!(self.is_ready().await, Ok(true)) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(Error::Timeout { timeout });
            }
            sleep(interval).await;
        }
    }
}
