//! Streaming response endpoints.
//!
//! BentoML streaming endpoints return the response body as a sequence of chunks.
//! The transport does not impose SSE or other framing: a `Generator[str]` endpoint
//! streams raw text, a model endpoint streams its serialized chunks, and chunk
//! boundaries follow the network, not logical records. This trait exposes the raw
//! [`ByteStream`]; [`ByteStream::text`] and [`ByteStream::lines`] adapt it for the
//! common text and newline-delimited (e.g. JSONL) cases.

mod decode;

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_core::Stream;
use serde::Serialize;

pub use self::decode::{LineStream, TextStream};
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

impl ByteStream {
    /// Decodes each chunk as UTF-8 text.
    ///
    /// Chunks follow network boundaries, so a multi-byte character could in theory
    /// be split across two chunks; use [`lines`] when the endpoint emits
    /// newline-delimited records and you need whole logical units.
    ///
    /// [`lines`]: Self::lines
    pub fn text(self) -> TextStream {
        TextStream::new(self)
    }

    /// Yields one logical line per `\n`, buffering across chunk boundaries.
    ///
    /// The trailing newline is stripped (and a preceding `\r`, so CRLF works). A
    /// final unterminated line is emitted when the stream ends. Suited to JSONL and
    /// other newline-delimited streaming endpoints.
    pub fn lines(self) -> LineStream {
        LineStream::new(self)
    }
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
