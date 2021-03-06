//! `db` subcommand for interaction with the database.

use falconeri_common::{db, prelude::*};
use std::process;
use structopt::StructOpt;

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
    let url = db::database_url(ConnectVia::Proxy)?;
    let status = process::Command::new("psql")
        .arg(&url)
        .status()
        .context("error starting psql")?;
    if !status.success() {
        return Err(format_err!("error running psql with {:?}", url));
    }
    Ok(())
}

/// Print out the database URL.
fn run_url() -> Result<()> {
    let url = db::database_url(ConnectVia::Proxy)?;
    println!("{}", url);
    Ok(())
}
