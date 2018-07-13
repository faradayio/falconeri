//! `db` subcommand for interaction with the database.

use failure::ResultExt;
use falconeri_common::{db, Result};
use std::process;

/// Commands for interacting with the database.
#[derive(Debug, StructOpt)]
pub enum Opt {
    /// Access the database console.
    #[structopt(name = "console")]
    Console,
    /// Print our a URL for connecting to the database.
    #[structopt(name = "url")]
    Url,
}

/// Run the `db` subcommand.
pub fn run(opt: &Opt) -> Result<()> {
    match opt {
        Opt::Console => run_console(),
        Opt::Url => run_url(),
    }
}

/// Connect to the database console.
fn run_console() -> Result<()> {
    let url = db::database_url(db::ConnectVia::Proxy)?;
    let status = process::Command::new("psql")
        .arg(&url)
        .status()
        .with_context(|_| format!("error starting psql"))?;
    if !status.success() {
        return Err(format_err!("error running psql with {:?}", url));
    }
    Ok(())
}

/// Print out the database URL.
fn run_url() -> Result<()> {
    let url = db::database_url(db::ConnectVia::Proxy)?;
    println!("{}", url);
    Ok(())
}
