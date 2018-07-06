//! Database utilities.

use Result;
use diesel::{PgConnection, prelude::*};
use failure::ResultExt;
use std::env;

/// Connect to PostgreSQL.
pub fn connect() -> Result<PgConnection> {
    let database_url = env::var("DATABASE_URL")
        .context("Can't read DATABASE_URL")?;
    let conn = PgConnection::establish(&database_url)
        .context("Error connecting to DATABASE_URL")?;
    Ok(conn)
}
