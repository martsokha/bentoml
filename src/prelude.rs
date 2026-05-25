//! Convenient re-exports of the most commonly used items.
//!
//! ```
//! use bentoml::prelude::*;
//! ```

pub use crate::error::{Error, Result};
#[cfg(feature = "stream")]
pub use crate::service::Streaming;
pub use crate::service::{Files, Readiness, Tasks};
pub use crate::{Client, ClientBuilder, Endpoint};
