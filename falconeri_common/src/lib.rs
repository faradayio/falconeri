//! Code shared between various Falconeri tools.

#![warn(missing_docs)]
// Silence diesel warnings: https://github.com/diesel-rs/diesel/pull/1787
#![allow(proc_macro_derive_resolution_fallback)]

// Keep `macro_use` for `diesel` until it's easier to use Rust 2018 macro
// imports with it.
#[macro_use]
pub extern crate diesel;
#[macro_use]
pub extern crate diesel_migrations;

pub use cast;
pub use chrono;
pub use common_failures;
pub use rand;
pub use semver;
pub use serde_json;

pub mod connect_via;
pub mod db;
pub mod kubernetes;
pub mod manifest;
pub mod models;
pub mod pipeline;
pub mod rest_api;
mod schema;
pub mod secret;
pub mod storage;

/// Common imports used by many modules.
pub mod prelude {
    pub use chrono::{NaiveDateTime, Utc};
    pub use diesel::{self, prelude::*, PgConnection};
    pub use failure::{format_err, ResultExt};
    pub use log::{debug, error, info, trace, warn};
    pub use serde::{Deserialize, Serialize};
    pub use std::{
        collections::HashMap,
        fmt,
        fs::File,
        io::Write,
        path::{Path, PathBuf},
    };
    pub use uuid::Uuid;

    pub use super::connect_via::ConnectVia;
    pub use super::models::*;
    pub use super::{Error, Result};
}

/// Error type for this crate's functions.
pub type Error = failure::Error;

/// Result type for this crate's functions.
pub type Result<T> = ::std::result::Result<T, Error>;

/// The version of `falconeri_common` that we're using. This can be used
/// to make sure that our various clients and servers match.
pub fn falconeri_common_version() -> semver::Version {
    env!("CARGO_PKG_VERSION")
        .parse::<semver::Version>()
        .expect("could not parse built-in version")
}
