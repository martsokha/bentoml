//! Request bodies: [`Input`] and the [`Multipart`] builder.
//!
//! BentoML keys an endpoint's input on the handler's parameter names, so a JSON
//! input is a struct with one field per parameter — even when there is only one:
//!
//! | Handler                             | Input                      |
//! |-------------------------------------|----------------------------|
//! | `f(self, text: str, top_k: int)`    | `&req` / [`Input::object`] |
//! | `f(self, requests: list[Req])`      | `&req` / [`Input::object`] |
//! | `f(self, requests: list[Req], /)`   | [`Input::root`]            |
//! | `f(self, data: bytes, /)`           | [`Input::bytes`]      |
//! | `f(self, image: Image, top_k: int)` | [`Multipart`]         |

mod multipart;

use bytes::Bytes;
use reqwest_middleware::RequestBuilder;
use serde::Serialize;
use serde_json::value::RawValue;

pub use self::multipart::{Multipart, Part};
use crate::error::{Error, Result};

/// The body an endpoint is called with; see the [module docs](self).
///
/// Construction is infallible: encoding errors surface as [`Error::InvalidRequest`]
/// when the request is made, before anything is sent.
#[derive(Debug)]
#[must_use]
pub struct Input {
    body: Result<Body>,
}

/// An encoded request body.
#[derive(Debug)]
enum Body {
    Json(Box<RawValue>),
    Bytes(Bytes),
    Multipart(Multipart),
}

impl Input {
    /// The whole input model: a JSON object with one key per handler parameter,
    /// typically a struct mirroring the signature. Anything else is rejected.
    pub fn object<T>(input: &T) -> Self
    where
        T: Serialize + ?Sized,
    {
        let body = encode(input).and_then(|json| {
            // serde_json emits no leading whitespace, so the first byte is the value's.
            if json.get().starts_with('{') {
                Ok(Body::Json(json))
            } else {
                Err(Error::invalid_message(
                    "endpoint input must serialize to a JSON object keyed on the \
                     handler's parameter names: wrap it in a struct with one field \
                     per parameter, or use `Input::root` for a positional-only one",
                ))
            }
        });
        Self { body }
    }

    /// A positional-only parameter, sent bare.
    pub fn root<T>(value: &T) -> Self
    where
        T: Serialize + ?Sized,
    {
        Self {
            body: encode(value).map(Body::Json),
        }
    }

    /// A positional-only binary parameter, sent as raw bytes.
    pub fn bytes(body: impl Into<Bytes>) -> Self {
        Self {
            body: Ok(Body::Bytes(body.into())),
        }
    }

    /// A `multipart/form-data` body, for file or image parameters.
    pub fn multipart(body: Multipart) -> Self {
        Self {
            body: Ok(Body::Multipart(body)),
        }
    }

    /// Attaches this body to `req`, or returns the error recorded at construction.
    pub(crate) fn attach(self, req: RequestBuilder) -> Result<RequestBuilder> {
        Ok(match self.body? {
            // Through `json` so the content type is set only when a per-call header
            // has not already set one.
            Body::Json(json) => req.json(&json),
            Body::Bytes(bytes) => req.body(bytes),
            Body::Multipart(form) => req.multipart(form.into_form()?),
        })
    }
}

/// The endpoint's whole input model; see [`Input::object`].
impl<T> From<&T> for Input
where
    T: Serialize + ?Sized,
{
    fn from(input: &T) -> Self {
        Self::object(input)
    }
}

/// A `multipart/form-data` body; see [`Input::multipart`].
impl From<Multipart> for Input {
    fn from(body: Multipart) -> Self {
        Self::multipart(body)
    }
}

/// Serializes `value` to JSON.
fn encode<T>(value: &T) -> Result<Box<RawValue>>
where
    T: Serialize + ?Sized,
{
    serde_json::value::to_raw_value(value)
        .map_err(|e| Error::invalid_request("failed to serialize the JSON input", e))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// The encoded JSON of `input`, which must have encoded successfully.
    fn json(input: Input) -> String {
        match input.body.expect("encodes") {
            Body::Json(json) => json.get().to_owned(),
            body => panic!("expected a JSON body, got {body:?}"),
        }
    }

    #[test]
    fn object_accepts_an_object() {
        assert_eq!(
            json(Input::object(&json!({"text": "a"}))),
            r#"{"text":"a"}"#
        );
    }

    #[test]
    fn object_rejects_a_non_object() {
        for input in [json!([1, 2]), json!("a"), json!(1), json!(null)] {
            let err = Input::object(&input)
                .body
                .expect_err("a non-object is rejected");
            assert!(matches!(err, Error::InvalidRequest { .. }), "{err:?}");
        }
    }

    #[test]
    fn a_reference_converts_to_an_object() {
        let err = Input::from(&[1, 2])
            .body
            .expect_err("goes through `object`");
        assert!(matches!(err, Error::InvalidRequest { .. }), "{err:?}");
    }

    #[test]
    fn root_sends_the_value_bare() {
        assert_eq!(json(Input::root(&[1, 2])), "[1,2]");
    }
}
