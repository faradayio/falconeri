//! The `deploy` subcommand.

use base64;
use falconeri_common::{
    kubernetes,
    manifest::render_manifest,
    prelude::*,
    rand::{distributions::Alphanumeric, rngs::StdRng, Rng, SeedableRng},
};
use std::iter;
use structopt::StructOpt;

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
    /// The number of copies of `falconerid` to run.
    falconerid_replicas: u16,
    /// The amount of RAM to request for `falconerid`.
    falconerid_memory: String,
    /// The number of CPUs to request for `falconerid`.
    falconerid_cpu: String,
    /// Should we get our `falconeri` image from `minikube`'s internal Docker
    /// daemon?
    use_local_image: bool,
    /// The version of `falconeri`.
    version: String,
}

/// Parameters used to generate a deploy manifest.
#[derive(Serialize)]
struct DeployManifestParams {
    all: bool,
    config: Config,
}

/// Commands for interacting with the database.
#[derive(Debug, StructOpt)]
#[structopt(name = "deploy", about = "Commands for interacting with the database.")]
pub struct Opt {
    /// Just print out the manifest without deploying it.
    #[structopt(long = "dry-run")]
    dry_run: bool,

    /// Don't include a secret in the manifest.
    #[structopt(long = "skip-secret")]
    skip_secret: bool,

    /// Deploy a development server (for minikube).
    #[structopt(long = "development")]
    development: bool,

    /// The amount of disk to allocate for PostgreSQL.
    #[structopt(long = "postgres-storage")]
    postgres_storage: Option<String>,

    /// The amount of RAM to request for PostgreSQL.
    #[structopt(long = "postgres-memory")]
    postgres_memory: Option<String>,

    /// The number of CPUs to request for PostgreSQL.
    #[structopt(long = "postgres-cpu")]
    postgres_cpu: Option<String>,

    /// The number of copies of `falconerid` to run.
    #[structopt(long = "falconerid-replicas")]
    falconerid_replicas: Option<u16>,

    /// The amount of RAM to request for `falconerid`.
    #[structopt(long = "falconerid-memory")]
    falconerid_memory: Option<String>,

    /// The number of CPUs to request for `falconerid`.
    #[structopt(long = "falconerid-cpu")]
    falconerid_cpu: Option<String>,
}

/// Deploy `falconeri` to the current Kubernetes cluster.
pub fn run(opt: &Opt) -> Result<()> {
    // Generate a password using the system's "secure" random number generator.
    let mut rng = StdRng::from_entropy();
    let postgres_password = iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(32)
        .collect::<Vec<u8>>();

    // Generate our secret manifest.
    let secret_params = SecretManifestParams {
        postgres_password: base64::encode(&postgres_password),
    };
    let secret_manifest = render_manifest(SECRET_MANIFEST, &secret_params)?;

    // Figure out our configuration.
    let mut config = default_config(opt.development);
    if let Some(postgres_storage) = &opt.postgres_storage {
        config.postgres_storage = postgres_storage.to_owned();
    }
    if let Some(postgres_memory) = &opt.postgres_memory {
        config.postgres_memory = postgres_memory.to_owned();
    }
    if let Some(postgres_cpu) = &opt.postgres_cpu {
        config.postgres_cpu = postgres_cpu.to_owned();
    }
    if let Some(falconerid_replicas) = opt.falconerid_replicas {
        config.falconerid_replicas = falconerid_replicas;
    }
    if let Some(falconerid_memory) = &opt.falconerid_memory {
        config.falconerid_memory = falconerid_memory.to_owned();
    }
    if let Some(falconerid_cpu) = &opt.falconerid_cpu {
        config.falconerid_cpu = falconerid_cpu.to_owned();
    }

    // Generate our deploy manifest.
    let deploy_params = DeployManifestParams { all: true, config };
    let deploy_manifest = render_manifest(DEPLOY_MANIFEST, &deploy_params)?;

    // Combine our manifests, only including the secret if we need it.
    let mut manifest = String::new();
    if !opt.skip_secret && !kubernetes::resource_exists("secret/falconeri")? {
        manifest.push_str(&secret_manifest);
    }
    manifest.push_str(&deploy_manifest);

    if opt.dry_run {
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
            falconerid_replicas: 1,
            falconerid_memory: "64Mi".to_string(),
            falconerid_cpu: "100m".to_string(),
            use_local_image: true,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    } else {
        Config {
            env: "production".to_string(),
            postgres_storage: "10Gi".to_string(),
            postgres_memory: "1Gi".to_string(),
            postgres_cpu: "500m".to_string(),
            falconerid_replicas: 2,
            falconerid_memory: "256Mi".to_string(),
            falconerid_cpu: "450m".to_string(),
            use_local_image: false,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}
