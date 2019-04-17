//! The `deploy` subcommand.

use base64;
use falconeri_common::{
    kubernetes,
    prelude::*,
    rand::{distributions::Alphanumeric, rngs::EntropyRng, Rng},
};
use std::iter;

use crate::manifest::render_manifest;

/// The manifest defining secrets for `falconeri`.
const SECRET_MANIFEST: &str = include_str!("secret_manifest.yml.hbs");

/// The manifest we use to deploy `falconeri`.
const DEPLOY_MANIFEST: &str = include_str!("deploy_manifest.yml.hbs");

/// Parameters used to generate a secret manifest.
#[derive(Serialize)]
struct SecretManifestParams {
    postgres_password: String,
}

/// Per-environment configuration.
#[derive(Serialize)]
struct Config {
    /// The name of the environment. Should be `development` or `production`.
    env: String,
    /// The amount of disk to allocate for PostgreSQL.
    postgres_storage: String,
    /// The amount of RAM to request for PostgreSQL.
    postgres_memory: String,
    /// The number of CPUs to request for PostgreSQL.
    postgres_cpu: String,
    /// The amount of RAM to request for `falconerid`.
    falconerid_memory: String,
    /// The number of CPUs to request for `falconerid`.
    falconerid_cpu: String,
    /// Should we get our `falconeri` image from `minikube`'s internal Docker
    /// daemon?
    use_local_image: bool,
}

/// Parameters used to generate a deploy manifest.
#[derive(Serialize)]
struct DeployManifestParams {
    all: bool,
    config: Config,
}

/// Deploy `falconeri` to the current Kubernetes cluster.
pub fn run(dry_run: bool, development: bool) -> Result<()> {
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
    let deploy_params = DeployManifestParams {
        all: true,
        config: default_config(development),
    };
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
    let params = DeployManifestParams {
        all,
        // We can always use the production config, because we don't
        // care about the details of the resources we're deleting.
        config: default_config(false),
    };
    let manifest = render_manifest(DEPLOY_MANIFEST, &params)?;
    kubernetes::undeploy(&manifest)?;

    // Clean up our secrets manually instead of rending a new manifest.
    if all {
        kubernetes::delete("secret/falconeri")?;
    }

    Ok(())
}

/// Get our default deployment config.
fn default_config(development: bool) -> Config {
    if development {
        Config {
            env: "development".to_string(),
            postgres_storage: "100Mi".to_string(),
            postgres_memory: "256Mi".to_string(),
            postgres_cpu: "100m".to_string(),
            falconerid_memory: "64Mi".to_string(),
            falconerid_cpu: "100m".to_string(),
            use_local_image: true,
        }
    } else {
        Config {
            env: "production".to_string(),
            postgres_storage: "10Gi".to_string(),
            postgres_memory: "2Gi".to_string(),
            postgres_cpu: "900m".to_string(),
            falconerid_memory: "256Mi".to_string(),
            falconerid_cpu: "1".to_string(),
            use_local_image: true,
        }
    }
}
