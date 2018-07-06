extern crate env_logger;
extern crate falconeri_common;
#[macro_use]
extern crate log;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate structopt;

use std::path::PathBuf;
use structopt::StructOpt;

mod pipeline;

/// Command-line options, parsed using `structopt`.
#[derive(Debug, StructOpt)]
#[structopt(about = "A tool for running batch jobs on Kubernetes.")]
enum Opt {
    /// Run the specified pipeline.
    #[structopt(name = "run")]
    Run {
        /// Path to a JSON pipeline spec.
        #[structopt(parse(from_os_str))]
        pipeline_json: PathBuf,
    }
}

fn main() {
    env_logger::init();
    let opt = Opt::from_args();
    debug!("Args: {:?}", opt);
}
