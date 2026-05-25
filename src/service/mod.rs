//! Capability traits implemented for [`Client`].
//!
//! The core [`Client`] carries the generic [`call`] for JSON endpoints. The traits
//! here extend it with the rest of the BentoML HTTP surface — health checks, async
//! task queues, file and raw-binary I/O, and (behind the `stream` feature) streaming
//! responses. Bring them into scope via the [prelude].
//!
//! [`Client`]: crate::Client
//! [`call`]: crate::Client::call
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
pub use self::stream::{ByteStream, Streaming};
