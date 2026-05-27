//! The [`EndpointResponse`] returned by the endpoint `call_*` methods.

use bytes::Bytes;
use serde::de::DeserializeOwned;

use crate::error::Result;

/// A successful response from an endpoint, ready to be read in a chosen format.
///
/// The `call_*` methods on [`Endpoint`] return this so the request and response
/// encodings are chosen independently: send JSON, raw bytes, or a multipart body,
/// then read the response as [`json`], [`bytes`], or [`text`].
///
/// The non-success status is already mapped to an error before this is returned, so
/// reading it only fails on a decode/transport error.
///
/// [`Endpoint`]: crate::Endpoint
/// [`json`]: EndpointResponse::json
/// [`bytes`]: EndpointResponse::bytes
/// [`text`]: EndpointResponse::text
#[derive(Debug)]
pub struct EndpointResponse {
    inner: reqwest::Response,
}

impl EndpointResponse {
    pub(crate) fn new(inner: reqwest::Response) -> Self {
        Self { inner }
    }

    /// Deserializes the response body as JSON into `R`.
    pub async fn json<R: DeserializeOwned>(self) -> Result<R> {
        Ok(self.inner.json::<R>().await?)
    }

    /// Returns the raw response body, for file, image, or other binary output.
    pub async fn bytes(self) -> Result<Bytes> {
        Ok(self.inner.bytes().await?)
    }

    /// Returns the response body decoded as UTF-8 text.
    pub async fn text(self) -> Result<String> {
        Ok(self.inner.text().await?)
    }

    /// The HTTP status code of the response.
    pub fn status(&self) -> u16 {
        self.inner.status().as_u16()
    }

    /// Consumes this wrapper, returning the underlying [`reqwest::Response`].
    ///
    /// An escape hatch for response handling not covered here, such as reading
    /// headers or streaming the body manually.
    pub fn into_inner(self) -> reqwest::Response {
        self.inner
    }
}
