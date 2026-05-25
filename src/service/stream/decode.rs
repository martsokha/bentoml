//! Decoders that adapt a raw [`ByteStream`] into higher-level streams.

use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;

use super::ByteStream;
use crate::error::{Error, Result};

/// A [`Stream`] that decodes [`ByteStream`] chunks as UTF-8 [`String`]s.
#[derive(Debug)]
pub struct TextStream {
    inner: ByteStream,
}

impl TextStream {
    pub(super) fn new(inner: ByteStream) -> Self {
        Self { inner }
    }
}

impl Stream for TextStream {
    type Item = Result<String>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx).map(|opt| {
            opt.map(|res| {
                let bytes = res?;
                String::from_utf8(bytes.to_vec())
                    .map_err(|e| Error::Decode(format!("invalid utf-8 chunk: {e}")))
            })
        })
    }
}

/// A [`Stream`] that yields newline-delimited lines from a [`ByteStream`].
#[derive(Debug)]
pub struct LineStream {
    inner: ByteStream,
    buf: Vec<u8>,
    ready: VecDeque<String>,
    done: bool,
}

impl LineStream {
    pub(super) fn new(inner: ByteStream) -> Self {
        Self {
            inner,
            buf: Vec::new(),
            ready: VecDeque::new(),
            done: false,
        }
    }

    /// Splits any complete lines out of `buf` into `ready`, decoding each as UTF-8.
    fn drain_lines(&mut self) -> Result<()> {
        while let Some(nl) = self.buf.iter().position(|&b| b == b'\n') {
            let mut line: Vec<u8> = self.buf.drain(..=nl).collect();
            line.pop(); // drop '\n'
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            let line = String::from_utf8(line)
                .map_err(|e| Error::Decode(format!("invalid utf-8 line: {e}")))?;
            self.ready.push_back(line);
        }
        Ok(())
    }
}

impl Stream for LineStream {
    type Item = Result<String>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            if let Some(line) = self.ready.pop_front() {
                return Poll::Ready(Some(Ok(line)));
            }
            if self.done {
                // Flush a final unterminated line, if any.
                if self.buf.is_empty() {
                    return Poll::Ready(None);
                }
                let rest = std::mem::take(&mut self.buf);
                return match String::from_utf8(rest) {
                    Ok(s) => Poll::Ready(Some(Ok(s))),
                    Err(e) => {
                        Poll::Ready(Some(Err(Error::Decode(format!("invalid utf-8 line: {e}")))))
                    }
                };
            }

            match Pin::new(&mut self.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    self.buf.extend_from_slice(&chunk);
                    if let Err(e) = self.drain_lines() {
                        return Poll::Ready(Some(Err(e)));
                    }
                }
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
                Poll::Ready(None) => self.done = true,
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}
