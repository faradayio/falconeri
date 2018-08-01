//! Code shared between various Falconeri tools.

#![warn(missing_docs)]

extern crate base64;
pub extern crate cast;
pub extern crate chrono;
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
pub extern crate rand;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
pub extern crate serde_json;
extern crate uuid;

pub mod db;
pub mod kubernetes;
pub mod models;
#[allow(missing_docs, unused_imports)]
mod schema;
pub mod storage;

/// Common imports used by many modules.
pub mod prefix {
    pub use chrono::{NaiveDateTime, Utc};
    pub use diesel::{self, prelude::*, PgConnection};
    pub use failure::ResultExt;
    pub use serde::{Deserialize, Serialize};
    pub use std::{
        collections::HashMap, fmt, fs::File, io::Write, path::{Path, PathBuf},
    };
    pub use uuid::Uuid;

    pub use super::models::*;
    pub use super::{Error, Result};
}

/// Error type for this crate's functions.
pub type Error = failure::Error;

/// Result type for this crate's functions.
pub type Result<T> = ::std::result::Result<T, Error>;
