//! Database utilities.

use anyhow::anyhow;
use diesel::r2d2::ConnectionManager as DieselConnectionManager;
use diesel::sql_query;
use diesel::sql_types::BigInt;
use diesel_migrations::{HarnessWithOutput, MigrationHarness};
use r2d2;
use std::{env, fs::read_to_string, io};

use crate::kubernetes::{base64_encoded_secret_string, kubectl_secret};
use crate::prelude::*;

/// Embed our migrations directly into the executable. We use a
/// submodule so we can configure warnings.
#[allow(unused_imports)]
mod migrations {
    use diesel_migrations::{embed_migrations, EmbeddedMigrations};
    pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");
}

/// The data we store in our secret.
#[derive(Debug, Deserialize)]
struct FalconeriSecretData {
    #[serde(with = "base64_encoded_secret_string", rename = "POSTGRES_PASSWORD")]
    postgres_password: String,
}

/// Look up our PostgreSQL password in our cluster's `falconeri` secret.
#[tracing::instrument(level = "trace")]
pub fn postgres_password(via: ConnectVia) -> Result<String> {
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
#[tracing::instrument(level = "trace")]
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
            let host = env::var("FALCONERI_PROXY_HOST")
                .unwrap_or_else(|_| "localhost".to_string());
            Ok(format!("postgres://postgres:{}@{}:5432/", password, host))
        }
        ConnectVia::Cluster => Ok(format!(
            "postgres://postgres:{}@falconeri-postgres:5432/",
            password,
        )),
    }
}

/// Connect to PostgreSQL.
#[tracing::instrument(level = "trace")]
pub fn connect(via: ConnectVia) -> Result<PgConnection> {
    let database_url = database_url(via)?;

    let conn = via
        .retry_if_appropriate(|| Ok(PgConnection::establish(&database_url)?))
        .with_context(|| format!("Error connecting to {}", database_url))?;

    Ok(conn)
}

/// A database connection pool.
pub type Pool = r2d2::Pool<DieselConnectionManager<PgConnection>>;

/// A connection using our connection pool.
pub type PooledConnection =
    r2d2::PooledConnection<DieselConnectionManager<PgConnection>>;

/// Create a connection pool using the specified parameters.
#[tracing::instrument(level = "trace")]
pub fn pool(pool_size: u32, via: ConnectVia) -> Result<Pool> {
    let database_url = database_url(via)?;
    let manager = DieselConnectionManager::new(database_url);
    let pool = r2d2::Pool::builder()
        .max_size(pool_size)
        .build(manager)
        .context("could not create database pool")?;
    Ok(pool)
}

/// The ID of the advisory lock that we use for migrations. Random.
const MIGRATION_LOCK_ID: i64 = 5_275_218_930_720_578_783;

/// Run any pending migrations, and print to standard output.
#[tracing::instrument(skip(conn), level = "trace")]
pub fn run_pending_migrations(conn: &mut PgConnection) -> Result<()> {
    debug!("Running pending migrations");
    conn.transaction(|conn| -> Result<()> {
        // Take an advisory lock before running the migration. It's safe to
        // generate this SQL by hand because MIGRATION_LOCK_ID is an integer.
        let lock_sql = sql_query("SELECT pg_advisory_xact_lock(?)");
        lock_sql
            .bind::<BigInt, _>(MIGRATION_LOCK_ID)
            .execute(conn)
            .context("error taking advisory lock for migrations")?;

        let mut stdout = io::stdout();
        let mut harness = HarnessWithOutput::new(conn, &mut stdout);
        harness
            .run_pending_migrations(migrations::MIGRATIONS)
            .map_err(|e| anyhow!("could not run migrations: {}", e))?;
        Ok(())
    })?;
    Ok(())
}
