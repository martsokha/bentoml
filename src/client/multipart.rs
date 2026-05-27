//! The [`Multipart`] body builder.

use bytes::Bytes;
use reqwest::multipart::{Form, Part};
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
/// use bentoml::Multipart;
///
/// # fn build(image: Vec<u8>) -> Multipart {
/// Multipart::new()
///     .field("prompt", &"a bento box")
///     .part("image", image, "image.jpg", "image/jpeg")
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

    /// Adds a file part with the given bytes, file name, and MIME type.
    pub fn part(
        mut self,
        name: impl Into<String>,
        bytes: impl Into<Bytes>,
        file_name: impl Into<String>,
        mime: impl AsRef<str>,
    ) -> Self {
        if self.error.is_none() {
            let part = Part::bytes(bytes.into().to_vec()).file_name(file_name.into());
            match part.mime_str(mime.as_ref()) {
                Ok(part) => self.parts.push((name.into(), part)),
                Err(e) => self.error = Some(format!("part {:?}: {e}", name.into())),
            }
        }
        self
    }

    /// Consumes the builder into a [`Form`], or returns the recorded error.
    pub(crate) fn into_form(self) -> Result<Form> {
        if let Some(error) = self.error {
            return Err(Error::InvalidMultipart(error));
        }
        let mut form = Form::new();
        for (name, value) in self.fields {
            form = form.text(name, value);
        }
        for (name, part) in self.parts {
            form = form.part(name, part);
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
