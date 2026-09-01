//! Scoping retries to the requests that can be replayed safely.

use reqwest::Response as ReqwestResponse;
use reqwest::{Method, Request};
use reqwest_middleware::{Middleware, Next, Result};
use reqwest_retry::RetryTransientMiddleware;
use reqwest_retry::policies::ExponentialBackoff;

/// Applies the wrapped retry middleware only to requests that can be replayed safely.
///
/// `reqwest-retry` classifies purely on the response status: a `POST` that timed out
/// or returned a 5xx is retried like any other request. Its [`RetryableStrategy`] sees
/// only the response, which does not carry the method, and it offers no per-request
/// opt-out, so the scoping has to happen a layer up.
///
/// It matters because BentoML's `POST` endpoints run inference and create tasks. A
/// retry landing after the server already accepted the first attempt bills a second
/// (potentially expensive) inference run, or leaves a duplicate task queued by
/// [`submit`]. Task `status` / `get` are `GET`s and `cancel` is a `PUT`, so the polling
/// path keeps its retries.
///
/// [`RetryableStrategy`]: reqwest_retry::RetryableStrategy
/// [`submit`]: crate::task::TaskEndpoint::submit
pub(crate) struct IdempotentOnly(RetryTransientMiddleware<ExponentialBackoff>);

impl IdempotentOnly {
    /// Wraps a retry middleware so it applies only to idempotent methods.
    pub(crate) fn new(inner: RetryTransientMiddleware<ExponentialBackoff>) -> Self {
        Self(inner)
    }
}

#[async_trait::async_trait]
impl Middleware for IdempotentOnly {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut http::Extensions,
        next: Next<'_>,
    ) -> Result<ReqwestResponse> {
        if is_idempotent(req.method()) {
            self.0.handle(req, extensions, next).await
        } else {
            next.run(req, extensions).await
        }
    }
}

/// Whether a method can be repeated without running the work a second time.
fn is_idempotent(method: &Method) -> bool {
    matches!(
        *method,
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE | Method::PUT | Method::DELETE
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_endpoints_that_run_inference_are_not_replayable() {
        // `call` and `submit` are POSTs: a replayed one runs inference twice, or
        // queues a duplicate task.
        assert!(!is_idempotent(&Method::POST));
        assert!(!is_idempotent(&Method::PATCH));

        // The task polling path stays retryable.
        assert!(is_idempotent(&Method::GET));
        assert!(is_idempotent(&Method::PUT));
        assert!(is_idempotent(&Method::DELETE));
    }
}
