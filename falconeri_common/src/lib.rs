//! Code shared between various Falconeri tools.

#![warn(missing_docs)]

extern crate chrono;
#[macro_use]
extern crate diesel;
extern crate failure;
extern crate uuid;

pub mod models;
mod schema;

/// Error type for this crate's functions.
pub type Error = failure::Error;

/// Result type for this crate's functions.
pub type Result<T> = ::std::result::Result<T, Error>;
