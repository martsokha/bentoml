//! The [`Multipart`] body builder and its [`Part`].

use bytes::Bytes;
use reqwest::multipart::{Form, Part as ReqwestPart};
use serde::Serialize;

use crate::error::{Error, Result};

/// A `multipart/form-data` body for endpoints that take file or image inputs.
///
/// BentoML maps each parameter to its own form field: non-file parameters are sent
/// as JSON-encoded text fields (named by parameter name), and files as separate
/// file parts. This builder applies that encoding so callers keep their typed values
/// instead of hand-assembling a form.
///
/// Builder methods are infallible; a field that fails to serialize, or a part with
/// an invalid MIME type, surfaces as an error when the request is made.
///
/// ```no_run
/// use bentoml::multipart::{Multipart, Part};
///
/// # fn build(image: Vec<u8>) -> Multipart {
/// Multipart::new()
///     .field("prompt", &"a bento box")
///     .part("image", Part::new(image).with_file_name("image.jpg").with_mime("image/jpeg"))
/// # }
/// ```
#[derive(Default)]
#[must_use]
pub struct Multipart {
    fields: Vec<(String, String)>,
    parts: Vec<(String, Part)>,
    error: Option<String>,
}

impl Multipart {
    /// Creates an empty multipart body.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a non-file parameter, JSON-encoded into its own form field.
    ///
    /// This matches how BentoML expects scalar/structured parameters in a multipart
    /// request (each `json.dumps`'d into a field named by the parameter).
    pub fn field<T>(mut self, name: impl Into<String>, value: &T) -> Self
    where
        T: Serialize + ?Sized,
    {
        if self.error.is_none() {
            match serde_json::to_string(value) {
                Ok(json) => self.fields.push((name.into(), json)),
                Err(e) => self.error = Some(format!("field {:?}: {e}", name.into())),
            }
        }
        self
    }

    /// Adds a file [`Part`] under the given parameter name.
    pub fn part(mut self, name: impl Into<String>, part: Part) -> Self {
        self.parts.push((name.into(), part));
        self
    }

    /// Consumes the builder into a [`Form`], or returns the recorded error.
    pub(crate) fn into_form(self) -> Result<Form> {
        if let Some(error) = self.error {
            return Err(Error::invalid_message(format!(
                "invalid multipart body: {error}"
            )));
        }
        let mut form = Form::new();
        for (name, value) in self.fields {
            form = form.text(name, value);
        }
        for (name, part) in self.parts {
            form = form.part(name, part.into_reqwest()?);
        }
        Ok(form)
    }
}

impl std::fmt::Debug for Multipart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Multipart")
            .field("fields", &self.fields.len())
            .field("parts", &self.parts.len())
            .field("error", &self.error)
            .finish()
    }
}

/// A single file part of a [`Multipart`] body.
///
/// Created from its bytes with [`Part::new`]; the file name and MIME type are
/// optional metadata set with [`with_file_name`] and [`with_mime`].
///
/// [`with_file_name`]: Part::with_file_name
/// [`with_mime`]: Part::with_mime
#[derive(Debug, Clone)]
#[must_use]
pub struct Part {
    bytes: Bytes,
    file_name: Option<String>,
    mime: Option<String>,
}

impl Part {
    /// Creates a part from its raw bytes.
    pub fn new(bytes: impl Into<Bytes>) -> Self {
        Self {
            bytes: bytes.into(),
            file_name: None,
            mime: None,
        }
    }

    /// Sets the part's file name.
    pub fn with_file_name(mut self, file_name: impl Into<String>) -> Self {
        self.file_name = Some(file_name.into());
        self
    }

    /// Sets the part's MIME type, e.g. `"image/jpeg"`.
    pub fn with_mime(mut self, mime: impl Into<String>) -> Self {
        self.mime = Some(mime.into());
        self
    }

    /// Builds the underlying reqwest part, validating the MIME type if set.
    fn into_reqwest(self) -> Result<ReqwestPart> {
        let mut part = ReqwestPart::bytes(self.bytes.to_vec());
        if let Some(file_name) = self.file_name {
            part = part.file_name(file_name);
        }
        if let Some(mime) = self.mime {
            part = part
                .mime_str(&mime)
                .map_err(|e| Error::invalid_request(format!("invalid part mime {mime:?}"), e))?;
        }
        Ok(part)
    }
}
