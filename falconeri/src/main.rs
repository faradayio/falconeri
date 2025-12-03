use std::{io::stderr, process};

use clap::Parser;
use falconeri_common::prelude::*;

mod cmd;
mod description;

/// Command-line options.
#[derive(Debug, Parser)]
#[command(about = "A tool for running batch jobs on Kubernetes.")]
enum Opt {
    /// Datum-related commands.
    #[command(name = "datum")]
    Datum {
        #[command(subcommand)]
        cmd: cmd::datum::Opt,
    },

    /// Commands for accessing the database.
    #[command(name = "db")]
    Db {
        #[command(subcommand)]
        cmd: cmd::db::Opt,
    },

    /// Deploy falconeri onto the current Docker cluster.
    #[command(name = "deploy")]
    Deploy {
        #[command(flatten)]
        cmd: cmd::deploy::Opt,
    },

    /// Job-related commands.
    #[command(name = "job")]
    Job {
        #[command(subcommand)]
        cmd: cmd::job::Opt,
    },

    /// Manaually migrate falconeri's database schema to the latest version.
    #[command(name = "migrate")]
    Migrate,

    /// Create a proxy connection to the default Kubernetes cluster.
    #[command(name = "proxy")]
    Proxy,

    /// Undeploy `falconeri`, removing it from the cluster.
    #[command(name = "undeploy")]
    Undeploy {
        /// Also delete the database volume and the secrets.
        #[arg(long = "all")]
        all: bool,
    },
}

/// Wrapper around `run` which reports errors.
fn main() {
    if let Err(err) = run() {
        let stderr = stderr();
        write!(&mut stderr.lock(), "{}", err.display_causes_and_backtrace())
            .expect("Error occurred while trying to display error");
        process::exit(1);
    }
}

/// The actual main code of the application.
fn run() -> Result<()> {
    let opt = Opt::parse();
    debug!("Args: {:?}", opt);

    match opt {
        Opt::Datum { ref cmd } => cmd::datum::run(cmd),
        Opt::Db { ref cmd } => cmd::db::run(cmd),
        Opt::Deploy { ref cmd } => cmd::deploy::run(cmd),
        Opt::Job { ref cmd } => cmd::job::run(cmd),
        Opt::Migrate => cmd::migrate::run(),
        Opt::Proxy => cmd::proxy::run(),
        Opt::Undeploy { all } => cmd::deploy::run_undeploy(all),
    }
}
