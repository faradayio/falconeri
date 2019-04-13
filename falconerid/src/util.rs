//! Various Rocket-related utilities.

use falconeri_common::{db, prelude::*};
use rocket::{
    fairing,
    http::{RawStr, Status},
    logger,
    request::{self, FromParam, FromRequest, Request},
    Outcome, State,
};
use std::{ops, result};

/// Wrap `Uuid` in a type that can be deserialized by Rocket.
///
/// This is basically the same as `rocket_contrib::uuid::Uuid`, but it uses
/// an earlier version of the `uuid` crate that's compatible with `diesel`.
pub struct UuidParam(Uuid);

impl UuidParam {
    /// Return our underlying UUID.
    pub fn into_inner(&self) -> Uuid {
        self.0
    }
}

impl<'r> FromParam<'r> for UuidParam {
    // It seems to be customary to return the original input string as an error?
    type Error = &'r RawStr;

    fn from_param(param: &'r RawStr) -> result::Result<Self, Self::Error> {
        param
            .percent_decode()
            // Turn errors straight into our original `RawStr`.
            .map_err(|_| param)
            .and_then(|decoded| decoded.parse::<Uuid>().map_err(|_| param))
            .map(UuidParam)
    }
}

/// A connection to our database, using a connection pool.
///
/// This integrates with various Rocket magic, allowing Rocket to manage the
/// global connection pool and automatically check out connections for handlers
/// that need them.
///
/// Normally, Rocket would do all this for us using some handy libraries in
/// `rocket_contrib`, but we want to create our own connection pooling so we can
/// intergrate better with our Kubernetes cluster setup, so we roll our own.
///
/// This is heavily based on [this code][dbcodegen].
///
/// [dbcodegen]: https://github.com/SergioBenitez/Rocket/blob/master/contrib/codegen/src/database.rs
pub struct DbConn(db::PooledConnection);

impl DbConn {
    /// Return a "fairing" which can be used to attach a connection pool to a
    /// Rocket server.
    pub fn fairing() -> impl fairing::Fairing {
        fairing::AdHoc::on_attach("DbConn", |rocket| {
            match db::pool(rocket.config().workers.into(), ConnectVia::Cluster) {
                Ok(pool) => Ok(rocket.manage(DbPool(pool))),
                Err(err) => {
                    logger::error("failed to initialize database pool");
                    logger::error_(&format!("{:?}", err));
                    Err(rocket)
                }
            }
        })
    }
}

// Rocket uses this to fetch `DdConn` parameters from the HTTP request
// automatically.
impl<'a, 'r> FromRequest<'a, 'r> for DbConn {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, ()> {
        // Try to get the connection pool attached to our server.
        let pool = request.guard::<State<DbPool>>()?;

        // Get a connection.
        match pool.0.get() {
            Ok(conn) => Outcome::Success(DbConn(conn)),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ())),
        }
    }
}

// Transparently unwrap `DbConn` into `&PgConnection` when possible.
impl ops::Deref for DbConn {
    type Target = PgConnection;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for DbConn {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// This holds a `db::Pool` and it can be attached to a Rocket server.
struct DbPool(db::Pool);
