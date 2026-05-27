//! Convenient re-exports of the most commonly used items.
//!
//! ```
//! use bentoml::prelude::*;
//! ```

pub use crate::error::{Error, Result};
#[cfg(feature = "stream")]
pub use crate::stream::Streaming;
pub use crate::{Client, ClientBuilder, Endpoint, Multipart, Response};
