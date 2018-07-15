//! Database models.

use diesel::{deserialize, pg::Pg, serialize};

use prefix::*;

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
    #[postgres(type_name = "status")]
    pub struct Status;
}

/// Possible status values.
#[derive(AsExpression, Debug, Clone, Copy, Eq, FromSqlRow, Ord, PartialEq, PartialOrd, Serialize)]
#[sql_type = "sql_types::Status"]
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

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
    fn to_sql<W: Write>(
        &self,
        out: &mut serialize::Output<W, Pg>,
    ) -> serialize::Result {
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
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        match not_none!(bytes) {
            b"ready" => Ok(Status::Ready),
            b"running" => Ok(Status::Running),
            b"done" => Ok(Status::Done),
            b"error" => Ok(Status::Error),
            b"canceled" => Ok(Status::Canceled),
            _ => Err(format!("Unrecognized status value from database").into()),
        }
    }
}
