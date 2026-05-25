//! An unofficial async Rust client for [BentoML] services.
//!
//! BentoML services expose their `@bentoml.api` methods as HTTP `POST` endpoints
//! whose route is derived from the method name (e.g. a `summarize` method becomes
//! `/summarize`). Because endpoints are defined dynamically per-service, this crate
//! does not generate typed bindings. Instead it offers a generic
//! [`Client::call`](crate::client::Client::call) over [`serde`] types: you describe
//! the request and response shapes, and the client handles serialization, transport,
//! and error mapping.
//!
//! # Example
//!
//! ```no_run
//! use bentoml::prelude::*;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize)]
//! struct SummarizeRequest {
//!     text: String,
//! }
//!
//! #[derive(Deserialize)]
//! struct SummarizeResponse {
//!     summary: String,
//! }
//!
//! # async fn run() -> Result<()> {
//! let client = Client::builder()
//!     .base_url("http://localhost:3000")?
//!     .build()?;
//!
//! if !client.is_ready().await? {
//!     eprintln!("service is not ready");
//! }
//!
//! let resp: SummarizeResponse = client
//!     .call("summarize", &SummarizeRequest { text: "...".into() })
//!     .await?;
//!
//! println!("{}", resp.summary);
//! # Ok(())
//! # }
//! ```
//!
//! [BentoML]: https://www.bentoml.com

#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod client;
pub mod error;
pub mod model;
pub mod service;

pub mod prelude;

pub use crate::client::Client;
pub use crate::error::{Error, Result};
