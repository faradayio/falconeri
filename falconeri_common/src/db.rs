//! Database utilities.

use std::{env, fs::read_to_string, io};

use kubernetes::{base64_encoded_secret_string, kubectl_secret};
use prefix::*;

/// Embed our migrations directly into the executable. We use a
/// submodule so we can configure warnings.
#[allow(unused_imports)]
mod migrations {
    embed_migrations!();

    // Re-export everything because it's private.
    pub use self::embedded_migrations::*;
}

/// How should we connect to the database?
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectVia {
    /// Assume we're connecting via a `kubectl proxy`.
    Proxy,
    /// Assume we're connecting via internal cluster networking and DNS.
    Cluster,
}

/// The data we store in our secret.
#[derive(Debug, Deserialize)]
struct FalconeriSecretData {
    #[serde(with = "base64_encoded_secret_string", rename = "POSTGRES_PASSWORD")]
    postgres_password: String,
}

/// Look up our PostgreSQL password in our cluster's `falconeri` secret.
fn postgres_password(via: ConnectVia) -> Result<String> {
    match via {
        ConnectVia::Proxy => {
            trace!("Fetching POSTGRES_PASSWORD from secret `falconeri`");
            // We implement the following as Rust:
            //
            // kubectl get secret falconeri -o json |
            //     jq -r .data.POSTGRES_PASSWORD |
            //     base64 --decode
            let secret_data: FalconeriSecretData = kubectl_secret("falconeri")?;
            Ok(secret_data.postgres_password)
        }
        ConnectVia::Cluster => {
            // This should be mounted into our container.
            Ok(read_to_string("/etc/falconeri/secrets/POSTGRES_PASSWORD")
                .context("could not read /etc/falconeri/secrets/POSTGRES_PASSWORD")?)
        }
    }
}

/// Get an appropriate database URL.
pub fn database_url(via: ConnectVia) -> Result<String> {
    // Check the environment first, so it can be overridden for testing outside
    // of a full Kubernetes setup.
    if let Ok(database_url) = env::var("DATABASE_URL") {
        return Ok(database_url);
    }

    // Build a URL.
    let password = postgres_password(via)?;
    match via {
        ConnectVia::Proxy => {
            Ok(format!("postgres://postgres:{}@localhost:5432/", password))
        }
        ConnectVia::Cluster => Ok(format!(
            "postgres://postgres:{}@falconeri-postgres:5432/",
            password,
        )),
    }
}

/// Connect to PostgreSQL.
pub fn connect(via: ConnectVia) -> Result<PgConnection> {
    let database_url = database_url(via)?;
    let conn = PgConnection::establish(&database_url)
        .with_context(|_| format!("Error connecting to {}", database_url))?;
    Ok(conn)
}

/// Run any pending migrations, and print to standard output.
pub fn run_pending_migrations(conn: &PgConnection) -> Result<()> {
    debug!("Running pending migrations");
    migrations::run_with_output(conn, &mut io::stdout())?;
    Ok(())
}
