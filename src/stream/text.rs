//! The [`TextStream`] decoder.

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
                    .map_err(|e| Error::decode("invalid utf-8 chunk", e))
            })
        })
    }
}
