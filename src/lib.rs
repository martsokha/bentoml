#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod client;
mod error;

pub mod model;
pub mod prelude;
pub mod service;

pub use crate::client::{
    Client, ClientBuilder, ClientBuilderError, ClientConfig, DEFAULT_BASE_URL, DEFAULT_MAX_RETRIES,
    DEFAULT_TIMEOUT,
};
pub use crate::error::{Error, Result};
