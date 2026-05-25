//! Capability traits for talking to a BentoML service.
//!
//! An [`Endpoint`] carries the generic [`call`] for JSON endpoints. The [`Tasks`],
//! [`Files`], and (behind the `stream` feature) `Streaming` traits extend it with
//! the rest of the BentoML HTTP surface; [`Readiness`] adds health checks on the
//! [`Client`]. Bring them into scope via the [prelude].
//!
//! [`Client`]: crate::Client
//! [`Endpoint`]: crate::Endpoint
//! [`call`]: crate::Endpoint::call
//! [prelude]: crate::prelude

mod files;
mod readiness;
mod task;

pub use self::files::Files;
pub use self::readiness::Readiness;
pub use self::task::{TaskHandle, Tasks};

#[cfg(feature = "stream")]
mod stream;

#[cfg(feature = "stream")]
#[cfg_attr(docsrs, doc(cfg(feature = "stream")))]
pub use self::stream::{ByteStream, LineStream, Streaming, TextStream};
