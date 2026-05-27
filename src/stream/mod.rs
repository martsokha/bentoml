//! Streaming response endpoints.
//!
//! BentoML streaming endpoints return the response body as a sequence of chunks.
//! The transport does not impose SSE or other framing: a `Generator[str]` endpoint
//! streams raw text, a `Generator[Model]` endpoint streams concatenated JSON values,
//! and chunk boundaries follow the network, not logical records. The [`stream`] call
//! returns the raw [`ByteStream`]; [`text`], [`lines`], and [`json`] adapt it for the
//! common text, newline-delimited, and JSON-object cases.
//!
//! [`stream`]: crate::Endpoint::stream
//! [`text`]: ByteStream::text
//! [`lines`]: ByteStream::lines
//! [`json`]: ByteStream::json

mod json;
mod line;
mod text;

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_core::Stream;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub use self::json::JsonStream;
pub use self::line::LineStream;
pub use self::text::TextStream;
use crate::client::Endpoint;
use crate::error::Result;

impl Endpoint {
    /// Invokes the streaming endpoint with the given JSON `payload`, returning a
    /// [`ByteStream`] over the response body chunks. Requires the `stream` feature.
    ///
    /// Decode the chunks with [`ByteStream::text`], [`ByteStream::lines`], or
    /// [`ByteStream::json`].
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, payload), fields(route = %self.route(), request_id = self.request_id()), err))]
    pub async fn stream<T>(&self, payload: &T) -> Result<ByteStream>
    where
        T: Serialize + ?Sized + Sync,
    {
        let req = self.request(self.route())?.json(payload);
        let resp = self.client().send(req).await?;
        Ok(ByteStream::new(resp.bytes_stream()))
    }
}

/// A [`Stream`] of response body chunks, with errors mapped to [`crate::Error`].
///
/// [`Stream`]: futures_core::Stream
pub struct ByteStream {
    inner: Pin<Box<dyn Stream<Item = reqwest::Result<Bytes>> + Send>>,
}

impl ByteStream {
    pub(super) fn new(inner: impl Stream<Item = reqwest::Result<Bytes>> + Send + 'static) -> Self {
        Self {
            inner: Box::pin(inner),
        }
    }

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

    /// Yields one deserialized `T` per JSON value in the stream.
    ///
    /// BentoML streams structured outputs (e.g. `Generator[Model]`) as concatenated
    /// JSON values with no delimiter; this parses them incrementally, buffering
    /// across chunk boundaries. Use this rather than [`lines`] for object-streaming
    /// endpoints, since they are not newline-delimited.
    ///
    /// [`lines`]: Self::lines
    pub fn json<T: DeserializeOwned>(self) -> JsonStream<T> {
        JsonStream::new(self)
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
