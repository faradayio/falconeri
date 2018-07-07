//! Code shared between various Falconeri tools.

#![warn(missing_docs)]

extern crate chrono;
#[macro_use]
pub extern crate diesel;
extern crate failure;
extern crate serde_json;
extern crate uuid;

pub mod db;
pub mod models;
#[allow(missing_docs, unused_imports)]
mod schema;

/// Error type for this crate's functions.
pub type Error = failure::Error;

/// Result type for this crate's functions.
pub type Result<T> = ::std::result::Result<T, Error>;
