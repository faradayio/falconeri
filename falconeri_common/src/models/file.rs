use chrono::NaiveDateTime;
use diesel::{self, PgConnection, prelude::*};
use failure::ResultExt;
use uuid::Uuid;

use Result;
use schema::*;

/// An input file which needs to be downloaded to the worker container.
#[derive(Debug, Queryable)]
pub struct File {
    /// The unique ID of this file.
    pub id: Uuid,
    /// When this record was created.
    pub created_at: NaiveDateTime,
    /// The ID of the datum to which this file belongs.
    pub datum_id: Uuid,
    /// The URI from which this file can be downloaded.
    pub uri: String,
    /// The local path to which this file should be downloaded.
    pub local_path: String,
}

/// Data required to create a new `File`.
#[derive(Debug, Insertable)]
#[table_name = "files"]
pub struct NewFile {
    /// The ID of the datum to which this file belongs.
    pub datum_id: Uuid,
    /// The URI from which this file can be downloaded.
    pub uri: String,
    /// The local path to which this file should be downloaded.
    pub local_path: String,
}

impl NewFile {
    /// Insert a new job into the database.
    pub fn insert(&self, conn: &PgConnection) -> Result<File> {
        Ok(diesel::insert_into(files::table)
            .values(self)
            .get_result(conn)
            .context("error inserting file")?)
    }
}
