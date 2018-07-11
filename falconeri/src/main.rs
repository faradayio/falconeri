extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate falconeri_common;
#[macro_use]
extern crate log;
extern crate openssl;
extern crate openssl_probe;
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

mod pipeline;
mod cmd;

/// Command-line options, parsed using `structopt`.
#[derive(Debug, StructOpt)]
#[structopt(about = "A tool for running batch jobs on Kubernetes.")]
enum Opt {
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
}

fn main() -> Result<()> {
    env_logger::init();
    openssl_probe::init_ssl_cert_env_vars();
    let opt = Opt::from_args();
    debug!("Args: {:?}", opt);

    match opt {
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
    }
}
