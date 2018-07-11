//! The `deploy` subcommand.

use falconeri_common::{kubernetes, Result};

/// The manifest we use to deploy `falconeri`.
const DEPLOY_MANIFEST: &str = include_str!("deploy_manifest.yml");

/// Deploy `falconeri` to the current Kubernetes cluster.
pub fn run(dry_run: bool) -> Result<()> {
    if dry_run {
        print!("{}", DEPLOY_MANIFEST);
    } else {
        kubernetes::deploy(DEPLOY_MANIFEST)?;
    }
    Ok(())
}

/// Undeploy `falconeri`, removing it from the cluster.
pub fn run_undeploy(all: bool) -> Result<()> {
    kubernetes::undeploy(DEPLOY_MANIFEST)?;

    // Clean up storage resources that weren't directly created by our manifest.
    if all {
        kubernetes::delete("secret/falconeri")?;
        kubernetes::delete("pv/falconeri-postgres")?;
    }

    Ok(())
}
