//! The `migrate` subcommand.

use falconeri_common::db;

use Result;

/// Run the `migrate` subcommand.
pub fn run() -> Result<()> {
    let conn = db::connect(db::ConnectVia::Proxy)?;
    db::run_pending_migrations(&conn)
}
