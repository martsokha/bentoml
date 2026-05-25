//! Streaming response endpoints.
//!
//! BentoML streaming endpoints return the response body as a sequence of chunks.
//! This trait exposes them as a [`Stream`] of [`Bytes`]; callers decode chunks
//! (e.g. UTF-8 text) as appropriate for the endpoint.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_core::Stream;
use serde::Serialize;

use crate::client::Client;
use crate::error::Result;

/// Streaming-response operations against a BentoML service.
///
/// Implemented for [`Client`]. Requires the `stream` feature.
pub trait Streaming {
    /// Invokes the streaming endpoint at `route` with the given JSON `payload`,
    /// returning a [`Stream`] over the response body chunks.
    fn stream<T>(
        &self,
        route: &str,
        payload: &T,
    ) -> impl Future<Output = Result<ByteStream>> + Send
    where
        T: Serialize + ?Sized + Sync;
}

impl Streaming for Client {
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, payload), err))]
    async fn stream<T>(&self, route: &str, payload: &T) -> Result<ByteStream>
    where
        T: Serialize + ?Sized + Sync,
    {
        let req = self.post(route)?.json(payload);
        let resp = self.send(req).await?;
        Ok(ByteStream {
            inner: Box::pin(resp.bytes_stream()),
        })
    }
}

/// A [`Stream`] of response body chunks, with errors mapped to [`crate::Error`].
pub struct ByteStream {
    inner: Pin<Box<dyn Stream<Item = reqwest::Result<Bytes>> + Send>>,
}

impl Stream for ByteStream {
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner
            .as_mut()
            .poll_next(cx)
            .map(|opt| opt.map(|res| res.map_err(Into::into)))
    }
}

impl fmt::Debug for ByteStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ByteStream").finish_non_exhaustive()
    }
}
