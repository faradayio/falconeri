extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate falconeri_common;
extern crate handlebars;
#[macro_use]
extern crate log;
extern crate openssl;
extern crate openssl_probe;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate structopt;

use failure::ResultExt;
use falconeri_common::Result;
use std::{fs::File, path::PathBuf};
use structopt::StructOpt;

use pipeline::PipelineSpec;

mod cmd;
mod manifest;
mod pipeline;

/// Command-line options, parsed using `structopt`.
#[derive(Debug, StructOpt)]
#[structopt(about = "A tool for running batch jobs on Kubernetes.")]
enum Opt {
    /// Deploy falconeri onto the current Docker cluster.
    #[structopt(name = "deploy")]
    Deploy {
        /// Just print out the manifest without deploying it.
        #[structopt(long = "dry-run")]
        dry_run: bool,
    },

    /// Migrate falconeri's database schema to the latest version.
    #[structopt(name = "migrate")]
    Migrate,

    /// Create a proxy connection to the default Kubernetes cluster.
    #[structopt(name = "proxy")]
    Proxy,

    /// Run the specified pipeline.
    #[structopt(name = "run")]
    Run {
        /// Path to a JSON pipeline spec.
        #[structopt(parse(from_os_str))]
        pipeline_json: PathBuf,
    },

    /// Undeploy `falconeri`, removing it from the cluster.
    #[structopt(name = "undeploy")]
    Undeploy {
        /// Also delete the database volume and the secrets.
        #[structopt(long = "all")]
        all: bool,
    },
}

fn main() -> Result<()> {
    env_logger::init();
    openssl_probe::init_ssl_cert_env_vars();
    let opt = Opt::from_args();
    debug!("Args: {:?}", opt);

    match opt {
        Opt::Deploy { dry_run } => cmd::deploy::run(dry_run),
        Opt::Migrate => cmd::migrate::run(),
        Opt::Proxy => cmd::proxy::run(),
        Opt::Run { ref pipeline_json } => {
            let f = File::open(pipeline_json)
                .context("can't open pipeline JSON file")?;
            let pipeline_spec: PipelineSpec =
                serde_json::from_reader(f)
                .context("can't parse pipeline JSON file")?;
            cmd::run::run(&pipeline_spec)
        }
        Opt::Undeploy { all } => cmd::deploy::run_undeploy(all),
    }
}
