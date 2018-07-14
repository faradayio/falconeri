use chrono::NaiveDateTime;
use diesel::{self, PgConnection, prelude::*};
use failure::ResultExt;
use uuid::Uuid;

use Result;
use schema::*;
use super::Datum;

/// An input file which needs to be downloaded to the worker container.
#[derive(Associations, Debug, Identifiable, Queryable, Serialize)]
#[belongs_to(Datum, foreign_key = "datum_id")]
pub struct InputFile {
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

impl InputFile {
    /// Generate a sample value for testing.
    pub fn factory(datum: &Datum) -> Self {
        use chrono::Utc;
        let now = Utc::now().naive_utc();
        InputFile {
            id: Uuid::new_v4(),
            created_at: now,
            datum_id: datum.id,
            uri: "gs://example-bucket/input/file.csv".to_owned(),
            local_path: "/pfs/input/file.csv".to_owned(),
        }
    }
}

/// Data required to create a new `InputFile`.
#[derive(Debug, Insertable)]
#[table_name = "input_files"]
pub struct NewInputFile {
    /// The ID of the datum to which this file belongs.
    pub datum_id: Uuid,
    /// The URI from which this file can be downloaded.
    pub uri: String,
    /// The local path to which this file should be downloaded.
    pub local_path: String,
}

impl NewInputFile {
    /// Insert a new job into the database.
    pub fn insert(&self, conn: &PgConnection) -> Result<InputFile> {
        Ok(diesel::insert_into(input_files::table)
            .values(self)
            .get_result(conn)
            .context("error inserting input file")?)
    }
}
