// Needed for static linking to work right on Linux.
extern crate openssl_sys;

use std::{io::stderr, process};

use falconeri_common::prelude::*;
use structopt::StructOpt;

mod cmd;
mod description;

/// Command-line options, parsed using `structopt`.
#[derive(Debug, StructOpt)]
#[structopt(about = "A tool for running batch jobs on Kubernetes.")]
enum Opt {
    /// Datum-related commands.
    #[structopt(name = "datum")]
    Datum {
        #[structopt(subcommand)]
        cmd: cmd::datum::Opt,
    },

    /// Commands for accessing the database.
    #[structopt(name = "db")]
    Db {
        #[structopt(subcommand)]
        cmd: cmd::db::Opt,
    },

    /// Deploy falconeri onto the current Docker cluster.
    #[structopt(name = "deploy")]
    Deploy {
        #[structopt(flatten)]
        cmd: cmd::deploy::Opt,
    },

    /// Job-related commands.
    #[structopt(name = "job")]
    Job {
        #[structopt(subcommand)]
        cmd: cmd::job::Opt,
    },

    /// Manaually migrate falconeri's database schema to the latest version.
    #[structopt(name = "migrate")]
    Migrate,

    /// Create a proxy connection to the default Kubernetes cluster.
    #[structopt(name = "proxy")]
    Proxy,

    /// Undeploy `falconeri`, removing it from the cluster.
    #[structopt(name = "undeploy")]
    Undeploy {
        /// Also delete the database volume and the secrets.
        #[structopt(long = "all")]
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
    openssl_probe::init_ssl_cert_env_vars();
    let opt = Opt::from_args();
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
