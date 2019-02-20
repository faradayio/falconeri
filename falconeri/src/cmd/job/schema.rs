//! The `job schema` subcommand.

use falconeri_common::prelude::*;
use magnet_schema::BsonSchema;
use serde_json;
use std::io::stdout;

use crate::pipeline::PipelineSpec;

/// The `job schema` subcommand.
pub fn run() -> Result<()> {
    let bson = PipelineSpec::bson_schema();
    serde_json::to_writer_pretty(&mut stdout(), &bson)?;
    println!();
    Ok(())
}
