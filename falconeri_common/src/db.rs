//! Database utilities.

use Result;
use base64;
use diesel::{PgConnection, prelude::*};
use failure::ResultExt;
use serde_json;
use std::{env, fs::read_to_string, process::{Command, Stdio}};

/// How should we connect to the database?
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectVia {
    /// Assume we're connecting via a `kubectl proxy`.
    Proxy,
    /// Assume we're connecting via internal cluster networking and DNS.
    Cluster,
}

/// A Kubernetes secret (missing lots of fields).
#[derive(Debug, Deserialize)]
struct FalconeriSecret {
    data: FalconeriSecretData,
}

/// The data we store in our secret.
#[derive(Debug, Deserialize)]
struct FalconeriSecretData {
    #[serde(rename = "POSTGRES_PASSWORD")]
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
            let output = Command::new("kubectl")
                .args(&["get", "secret", "falconeri", "-o", "json"])
                // Pass `stderr` through on console instead of capturing.
                .stderr(Stdio::inherit())
                .output()
                .context("could not fetch POSTGRES_PASSWORD from secret `falconeri`")?;
            let secret: FalconeriSecret = serde_json::from_slice(&output.stdout)
                .context("error parsing secret `falconeri`")?;
            let pw_bytes = base64::decode(&secret.data.postgres_password)
                .context("cannot decode POSTGRES_PASSWORD")?;
            Ok(String::from_utf8(pw_bytes)
                .context("POSTGRES_PASSWORD must be valid UTF-8")?)
        }
        ConnectVia::Cluster => {
            // This should be mounted into our container.
            Ok(read_to_string("/etc/falconeri/secrets/POSTGRES_PASSWORD")
                .context("could not read /etc/falconeri/secrets/POSTGRES_PASSWORD")?)
        }
    }
}

/// Get an appropriate database URL.
fn database_url(via: ConnectVia) -> Result<String> {
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
        ConnectVia::Cluster => {
            Ok(format!("postgres://postgres:{}@falconeri-postgres:5432/", password))
        }
    }
}

/// Connect to PostgreSQL.
pub fn connect(via: ConnectVia) -> Result<PgConnection> {
    let database_url = database_url(via)?;
    let conn = PgConnection::establish(&database_url)
        .with_context(|_| format!("Error connecting to {}", database_url))?;
    Ok(conn)
}
