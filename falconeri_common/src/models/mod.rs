//! Database models.

use diesel::backend::Backend;
use diesel::{deserialize, pg::Pg, serialize};

use crate::prelude::*;

mod datum;
mod input_file;
mod job;
mod output_file;

pub use self::datum::*;
pub use self::input_file::*;
pub use self::job::*;
pub use self::output_file::*;

/// Custom SQL types.
pub mod sql_types {
    /// A status enumeration type for use in Diesel's `table!` macro.
    #[derive(QueryId, SqlType)]
    #[diesel(postgres_type(name = "status"))]
    pub struct Status;
}

/// Possible status values.
#[derive(
    AsExpression,
    Debug,
    Deserialize,
    Clone,
    Copy,
    Eq,
    FromSqlRow,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
)]
#[diesel(sql_type = sql_types::Status)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// This record is ready to be processed.
    Ready,
    /// This record is currently being processed.
    Running,
    /// This record has been successfully processed.
    Done,
    /// This record could not be processed.
    Error,
    /// This record has been canceled, and further processing should be
    /// stopped as soon as convenient.
    Canceled,
}

impl Status {
    /// Return true if this has completed successfully, failed beyond recovery,
    /// or been cancelled.
    pub fn has_finished(self) -> bool {
        match self {
            Status::Ready | Status::Running => false,
            Status::Done | Status::Error | Status::Canceled => true,
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match *self {
            Status::Ready => "ready",
            Status::Running => "running",
            Status::Done => "done",
            Status::Error => "error",
            Status::Canceled => "canceled",
        };
        s.fmt(f)
    }
}

impl ::diesel::serialize::ToSql<sql_types::Status, Pg> for Status {
    fn to_sql(&self, out: &mut serialize::Output<'_, '_, Pg>) -> serialize::Result {
        match *self {
            Status::Ready => out.write_all(b"ready")?,
            Status::Running => out.write_all(b"running")?,
            Status::Done => out.write_all(b"done")?,
            Status::Error => out.write_all(b"error")?,
            Status::Canceled => out.write_all(b"canceled")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl ::diesel::deserialize::FromSql<sql_types::Status, Pg> for Status {
    fn from_sql(bytes: <Pg as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        match String::from_sql(bytes)?.as_str() {
            "ready" => Ok(Status::Ready),
            "running" => Ok(Status::Running),
            "done" => Ok(Status::Done),
            "error" => Ok(Status::Error),
            "canceled" => Ok(Status::Canceled),
            val => {
                Err(format!("Unrecognized status value from database: {}", val).into())
            }
        }
    }
}
