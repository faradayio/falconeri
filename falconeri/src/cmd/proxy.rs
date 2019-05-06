//! The `proxy` subcommand.

use crossbeam::scope;
use falconeri_common::{kubernetes, prelude::*};

/// Run our proxy.
pub fn run() -> Result<()> {
    scope(|scope| -> Result<()> {
        scope.spawn(|_| forward("svc/falconeri-postgres", "5432:5432"));
        scope.spawn(|_| forward("svc/falconerid", "8089:8089"));
        Ok(())
    })
    .expect("background scope panic")
}

fn forward(service: &str, port: &str) -> Result<()> {
    kubernetes::kubectl(&["port-forward", service, port])
}
