//! The `datum` subcommand.

use clap::Parser;
use falconeri_common::prelude::*;

mod describe;

/// `datum` options.
#[derive(Debug, Parser)]
pub enum Opt {
    /// Describe a specific job.
    #[command(name = "describe")]
    Describe {
        /// The UUID of the datum to describe.
        id: Uuid,
    },
}

/// Run the `job` subcommand.
pub fn run(opt: &Opt) -> Result<()> {
    match opt {
        Opt::Describe { id } => describe::run(*id),
    }
}
