//! Database models.

use diesel::{deserialize, pg::Pg, serialize};
use std::io::Write;

mod datum;
mod file;
mod job;

pub use self::datum::*;
pub use self::file::*;
pub use self::job::*;

/// Custom SQL types.
pub mod sql_types {
    /// A status enumeration type for use in Diesel's `table!` macro.
    #[derive(QueryId, SqlType)]
    #[postgres(type_name = "status")]
    pub struct Status;
}

/// Possible status values.
#[derive(AsExpression, Debug, Clone, Copy, Eq, FromSqlRow, PartialEq)]
#[sql_type = "sql_types::Status"]
pub enum Status {
    /// This record is still being created, and is not ready to process.
    Creating,
    /// This record is ready to be processed.
    Ready,
    /// This record is currently being processed.
    Running,
    /// This record has been successfully processed.
    Done,
    /// This record could not be processed.
    Error,
}

impl ::diesel::serialize::ToSql<sql_types::Status, Pg> for Status {
    fn to_sql<W: Write>(
        &self,
        out: &mut serialize::Output<W, Pg>,
    ) -> serialize::Result {
        match *self {
            Status::Creating => out.write_all(b"creating")?,
            Status::Ready => out.write_all(b"ready")?,
            Status::Running => out.write_all(b"running")?,
            Status::Done => out.write_all(b"done")?,
            Status::Error => out.write_all(b"error")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl ::diesel::deserialize::FromSql<sql_types::Status, Pg> for Status {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        match not_none!(bytes) {
            b"creating" => Ok(Status::Creating),
            b"ready" => Ok(Status::Ready),
            b"running" => Ok(Status::Running),
            b"done" => Ok(Status::Done),
            b"error" => Ok(Status::Error),
            _ => Err(format!("Unrecognized status value from database").into()),
        }
    }
}
