//! Code shared between various Falconeri tools.

#![warn(missing_docs)]

// Silence diesel warnings: https://github.com/diesel-rs/diesel/pull/1787
#![allow(proc_macro_derive_resolution_fallback)]

use backoff;
use base64;
#[macro_use]
extern crate bson;
pub use cast;
pub use chrono;
pub use common_failures;
#[macro_use]
pub extern crate diesel;
#[macro_use]
pub extern crate diesel_migrations;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate magnet_derive;
use magnet_schema;
pub use rand;


#[macro_use]
extern crate serde_derive;
pub use serde_json;


pub mod db;
pub mod kubernetes;
pub mod models;
#[allow(missing_docs, unused_imports)]
mod schema;
pub mod secret;
pub mod storage;

/// Common imports used by many modules.
pub mod prefix {
    pub use chrono::{NaiveDateTime, Utc};
    pub use diesel::{self, prelude::*, PgConnection};
    pub use failure::ResultExt;
    pub use serde::{Deserialize, Serialize};
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
