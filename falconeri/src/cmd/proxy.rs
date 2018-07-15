//! The `proxy` subcommand.

use falconeri_common::{kubernetes, prefix::*};

/// Run our proxy.
pub fn run() -> Result<()> {
    kubernetes::kubectl(&["port-forward", "svc/falconeri-postgres", "5432:5432"])
}
