//! The [`JsonStream`] decoder.

use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use serde::de::DeserializeOwned;

use super::ByteStream;
use crate::error::{Error, Result};

/// A [`Stream`] that yields one deserialized `T` per JSON value in a [`ByteStream`].
///
/// BentoML streams structured outputs as concatenated JSON values with no delimiter
/// (e.g. `{"i":0}{"i":1}`); this parses them incrementally via
/// [`serde_json::StreamDeserializer`], buffering a partial value across chunk
/// boundaries until it completes.
pub struct JsonStream<T> {
    inner: ByteStream,
    buf: Vec<u8>,
    ready: VecDeque<Result<T>>,
    done: bool,
}

impl<T> JsonStream<T> {
    pub(super) fn new(inner: ByteStream) -> Self {
        Self {
            inner,
            buf: Vec::new(),
            ready: VecDeque::new(),
            done: false,
        }
    }
}

impl<T: DeserializeOwned> JsonStream<T> {
    /// Parses all complete JSON values from the front of `buf`, queueing them in
    /// `ready` and discarding the consumed bytes. A trailing incomplete value is
    /// left in `buf` for the next chunk; a malformed value queues an error.
    fn drain_values(&mut self) {
        let mut iter = serde_json::Deserializer::from_slice(&self.buf).into_iter::<T>();
        loop {
            match iter.next() {
                Some(Ok(value)) => self.ready.push_back(Ok(value)),
                // A premature EOF means the trailing value is not yet complete; keep
                // the unconsumed bytes and wait for more.
                Some(Err(e)) if e.is_eof() => break,
                Some(Err(e)) => {
                    self.ready
                        .push_back(Err(Error::Decode(format!("invalid json value: {e}"))));
                    // Skip the rest of this buffer; on malformed input there is no
                    // reliable resync point.
                    let consumed = iter.byte_offset();
                    self.buf.drain(..consumed);
                    return;
                }
                None => break,
            }
        }
        let consumed = iter.byte_offset();
        self.buf.drain(..consumed);
    }
}

impl<T: DeserializeOwned + Unpin> Stream for JsonStream<T> {
    type Item = Result<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // `JsonStream<T>` is `Unpin` (the only pinned-relevant field, `ByteStream`,
        // is itself `Unpin`), so we can work with a plain `&mut Self`.
        let this = self.get_mut();
        loop {
            if let Some(value) = this.ready.pop_front() {
                return Poll::Ready(Some(value));
            }
            if this.done {
                // Non-whitespace bytes left over means a trailing incomplete value.
                if this.buf.iter().any(|b| !b.is_ascii_whitespace()) {
                    this.buf.clear();
                    return Poll::Ready(Some(Err(Error::Decode(
                        "trailing incomplete json value".to_owned(),
                    ))));
                }
                return Poll::Ready(None);
            }

            match Pin::new(&mut this.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    this.buf.extend_from_slice(&chunk);
                    this.drain_values();
                }
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
                Poll::Ready(None) => this.done = true,
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use serde::Deserialize;
    use tokio_stream::StreamExt;

    use super::*;

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct Chunk {
        i: u32,
    }

    /// Builds a JsonStream<Chunk> over the given raw byte chunks.
    fn json_stream(chunks: &[&[u8]]) -> JsonStream<Chunk> {
        let owned: Vec<reqwest::Result<Bytes>> = chunks
            .iter()
            .map(|c| Ok(Bytes::copy_from_slice(c)))
            .collect();
        ByteStream::new(tokio_stream::iter(owned)).json::<Chunk>()
    }

    async fn collect(mut stream: JsonStream<Chunk>) -> Vec<Result<Chunk>> {
        let mut out = Vec::new();
        while let Some(item) = stream.next().await {
            out.push(item);
        }
        out
    }

    #[tokio::test]
    async fn parses_concatenated_objects_split_across_chunks() {
        // BentoML concatenates objects with no delimiter; the middle object is split
        // across the chunk boundary, so the buffer must reassemble it.
        let stream = json_stream(&[br#"{"i":0}{"i"#, br#"":1}{"i":2}"#]);
        let got: Vec<u32> = collect(stream)
            .await
            .into_iter()
            .map(|r| r.unwrap().i)
            .collect();
        assert_eq!(got, vec![0, 1, 2]);
    }

    #[tokio::test]
    async fn trailing_incomplete_object_is_an_error() {
        // The first object completes and is yielded; the dangling `{"i":` left at
        // EOF surfaces as a decode error.
        let out = collect(json_stream(&[br#"{"i":7}{"i":"#])).await;
        assert_eq!(out[0].as_ref().unwrap(), &Chunk { i: 7 });
        assert!(out[1].is_err());
    }
}
