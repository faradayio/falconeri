//! The `migrate` subcommand.

use falconeri_common::{db, prelude::*};

/// Run the `migrate` subcommand.
pub fn run() -> Result<()> {
    let conn = db::connect(ConnectVia::Proxy)?;
    db::run_pending_migrations(&conn)
}
