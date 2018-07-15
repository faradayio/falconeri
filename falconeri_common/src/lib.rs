//! Code shared between various Falconeri tools.

#![warn(missing_docs)]

extern crate base64;
pub extern crate cast;
extern crate chrono;
#[macro_use]
pub extern crate diesel;
#[macro_use]
pub extern crate diesel_migrations;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
pub extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate uuid;

pub mod db;
pub mod kubernetes;
pub mod models;
#[allow(missing_docs, unused_imports)]
mod schema;
pub mod storage;

/// Error type for this crate's functions.
pub type Error = failure::Error;

/// Result type for this crate's functions.
pub type Result<T> = ::std::result::Result<T, Error>;
