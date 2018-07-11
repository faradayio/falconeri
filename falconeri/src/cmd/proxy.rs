//! The `proxy` subcommand.

use failure::ResultExt;
use std::process::Command;

use Result;

/// Run our proxy.
pub fn run() -> Result<()> {
    let status = Command::new("kubectl")
        .args(&["port-forward", "svc/falconeri-postgres", "5432:5432"])
        .status()
        .context("error running kubectl")?;
    if status.success() {
        Ok(())
    } else {
        Err(format_err!("error running kubectl proxy"))
    }
}
