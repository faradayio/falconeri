//! Code shared between various Falconeri tools.

#![warn(missing_docs, clippy::pendantic)]
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
pub use serde_json;

pub mod db;
pub mod kubernetes;
pub mod models;
mod schema;
pub mod secret;
pub mod storage;

/// Common imports used by many modules.
pub mod prefix {
    pub use chrono::{NaiveDateTime, Utc};
    pub use diesel::{self, prelude::*, PgConnection};
    pub use failure::{format_err, ResultExt};
    pub use log::{debug, error, info, trace, warn};
    pub use serde::{Deserialize, Serialize};
    pub use serde_derive::{Deserialize, Serialize};
    pub use std::{
        collections::HashMap,
        fmt,
        fs::File,
        io::Write,
        path::{Path, PathBuf},
    };
    pub use uuid::Uuid;

    pub use super::models::*;
    pub use super::{Error, Result};
}

/// Error type for this crate's functions.
pub type Error = failure::Error;

/// Result type for this crate's functions.
pub type Result<T> = ::std::result::Result<T, Error>;
