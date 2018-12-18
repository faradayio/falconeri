//! The `deploy` subcommand.

use base64;
use falconeri_common::{
    kubernetes, rand::{distributions::Alphanumeric, rngs::EntropyRng, Rng}, Result,
};
use std::iter;

use manifest::render_manifest;

/// The manifest defining secrets for `falconeri`.
const SECRET_MANIFEST: &str = include_str!("secret_manifest.yml.hbs");

/// The manifest we use to deploy `falconeri`.
const DEPLOY_MANIFEST: &str = include_str!("deploy_manifest.yml.hbs");

/// Parameters used to generate a secret manifest.
#[derive(Serialize)]
struct SecretManifestParams {
    postgres_password: String,
}

/// Parameters used to generate a deploy manifest.
#[derive(Serialize)]
struct DeployManifestParams {
    all: bool,
}

/// Deploy `falconeri` to the current Kubernetes cluster.
pub fn run(dry_run: bool) -> Result<()> {
    // Generate a password using the system's "secure" random number generator.
    let mut rng = EntropyRng::new();
    let postgres_password = iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(32)
        .collect::<String>();

    // Generate our secret manifest.
    let secret_params = SecretManifestParams {
        postgres_password: base64::encode(&postgres_password),
    };
    let secret_manifest = render_manifest(SECRET_MANIFEST, &secret_params)?;

    // Generate our deploy manifest.
    let deploy_params = DeployManifestParams { all: true };
    let deploy_manifest = render_manifest(DEPLOY_MANIFEST, &deploy_params)?;

    // Combine our manifests, only including the secret if we need it.
    let mut manifest = String::new();
    if !kubernetes::resource_exists("secret/falconeri")? {
        manifest.push_str(&secret_manifest);
    }
    manifest.push_str(&deploy_manifest);

    if dry_run {
        // Print out our manifests.
        print!("{}", manifest);
    } else {
        kubernetes::deploy(&manifest)?;
    }
    Ok(())
}

/// Undeploy `falconeri`, removing it from the cluster.
pub fn run_undeploy(all: bool) -> Result<()> {
    // Clean up things declared by our regular manifest.
    let params = DeployManifestParams { all };
    let manifest = render_manifest(DEPLOY_MANIFEST, &params)?;
    kubernetes::undeploy(&manifest)?;

    // Clean up our secrets manually instead of rending a new manifest.
    if all {
        kubernetes::delete("secret/falconeri")?;
    }

    Ok(())
}
