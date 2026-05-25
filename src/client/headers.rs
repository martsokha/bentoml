//! A small header accumulator that keeps builder methods infallible.
//!
//! Header names and values are parsed eagerly into a [`HeaderMap`]; an invalid one
//! is stashed and surfaced when the request is built, mirroring how
//! [`reqwest::RequestBuilder::header`] defers its error to `send`. This keeps
//! `with_header` returning `Self` while storing already-typed headers (no
//! re-validation per request).

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest_middleware::RequestBuilder;

use crate::error::{Error, Result};

/// Accumulated per-request headers, plus the first parse error if any.
#[derive(Debug, Clone, Default)]
pub(crate) struct Headers {
    map: HeaderMap,
    error: Option<String>,
}

impl Headers {
    /// Parses and inserts a header, recording the first failure rather than failing.
    pub(crate) fn insert(&mut self, name: impl AsRef<str>, value: impl AsRef<str>) {
        if self.error.is_some() {
            return;
        }
        let name = match HeaderName::from_bytes(name.as_ref().as_bytes()) {
            Ok(name) => name,
            Err(e) => {
                self.error = Some(format!("{:?}: {e}", name.as_ref()));
                return;
            }
        };
        match HeaderValue::from_str(value.as_ref()) {
            Ok(value) => {
                self.map.insert(name, value);
            }
            Err(e) => self.error = Some(format!("{name}: {e}")),
        }
    }

    /// Applies the accumulated headers to `req`, or returns the recorded parse error.
    pub(crate) fn apply(&self, mut req: RequestBuilder) -> Result<RequestBuilder> {
        if let Some(error) = &self.error {
            return Err(Error::InvalidHeader(error.clone()));
        }
        for (name, value) in &self.map {
            req = req.header(name.clone(), value.clone());
        }
        Ok(req)
    }
}
