//! File, multipart, and raw-binary I/O.
//!
//! BentoML endpoints that take file or image inputs expect `multipart/form-data`,
//! with each parameter as a form field and files as file parts. Endpoints with a
//! single positional ("root") input accept the raw bytes as the request body.
//! Endpoints that return files or images respond with a binary body.

use std::future::Future;

use bytes::Bytes;
use reqwest::multipart::Form;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::client::Client;
use crate::error::Result;

/// File and raw-binary operations against a BentoML service.
///
/// Implemented for [`Client`].
pub trait Files {
    /// Invokes `route` with a `multipart/form-data` body, for endpoints that take
    /// file or image inputs. Build the [`Form`] with text fields and file parts.
    fn call_multipart<R>(&self, route: &str, form: Form) -> impl Future<Output = Result<R>> + Send
    where
        R: DeserializeOwned;

    /// Invokes `route` with a raw byte body, for endpoints that take a single
    /// positional binary input. The response is deserialized as JSON.
    fn call_raw<R>(
        &self,
        route: &str,
        body: impl Into<Bytes> + Send,
    ) -> impl Future<Output = Result<R>> + Send
    where
        R: DeserializeOwned;

    /// Invokes `route` with the given JSON `payload`, returning the raw response
    /// body, for endpoints that return a file, image, or other binary output.
    fn call_bytes<T>(&self, route: &str, payload: &T) -> impl Future<Output = Result<Bytes>> + Send
    where
        T: Serialize + ?Sized + Sync;
}

impl Files for Client {
    async fn call_multipart<R>(&self, route: &str, form: Form) -> Result<R>
    where
        R: DeserializeOwned,
    {
        let req = self.post(route)?.multipart(form);
        Ok(self.send(req).await?.json().await?)
    }

    async fn call_raw<R>(&self, route: &str, body: impl Into<Bytes> + Send) -> Result<R>
    where
        R: DeserializeOwned,
    {
        let req = self.post(route)?.body(body.into());
        Ok(self.send(req).await?.json().await?)
    }

    async fn call_bytes<T>(&self, route: &str, payload: &T) -> Result<Bytes>
    where
        T: Serialize + ?Sized + Sync,
    {
        let req = self.post(route)?.json(payload);
        Ok(self.send(req).await?.bytes().await?)
    }
}
