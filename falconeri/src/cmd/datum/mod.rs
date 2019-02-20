//! The `datum` subcommand.

use falconeri_common::prelude::*;
use structopt::StructOpt;

mod describe;

/// `datum` options.
#[derive(Debug, StructOpt)]
pub enum Opt {
    /// Describe a specific job.
    #[structopt(name = "describe")]
    Describe {
        /// The UUID of the datum to describe.
        #[structopt(parse(try_from_str))]
        id: Uuid,
    },
}

/// Run the `job` subcommand.
pub fn run(opt: &Opt) -> Result<()> {
    match opt {
        Opt::Describe { id } => describe::run(*id),
    }
}
