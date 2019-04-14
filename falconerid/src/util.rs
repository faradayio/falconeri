//! Various Rocket-related utilities.

use falconeri_common::{db, prelude::*};
use headers::{authorization::Basic, Authorization, Header, HeaderValue};
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

/// The administrator password for `falconeri`. This is looked up once and
/// stored in our server state.
struct AdminPassword(String);

/// An authenticated user. For now, this carries no identity information,
/// because we only distinguish between "authenticated" and "not authenticated",
/// and we therefore just need a placeholder that represents authentication.
pub struct User;

impl User {
    /// Return a "fairing" which can be used to set up authentication.
    pub fn fairing() -> impl fairing::Fairing {
        fairing::AdHoc::on_attach("User", |rocket| {
            match db::postgres_password(ConnectVia::Cluster) {
                Ok(password) => Ok(rocket.manage(AdminPassword(password))),
                Err(err) => {
                    logger::error("failed to look up admin password");
                    logger::error_(&format!("{:?}", err));
                    Err(rocket)
                }
            }
        })
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for User {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, ()> {
        // Get our auth header.
        let auth = match basic_auth_from_request(request) {
            Ok(Some(auth)) => auth,
            Ok(None) => {
                // TODO: Should send `WWW-Authenticate: Basic
                // realm="falconeri"`.
                return Outcome::Failure((Status::Unauthorized, ()));
            }
            Err(_) => return Outcome::Failure((Status::BadRequest, ())),
        };

        // Get the admin password for our server.
        let password = request.guard::<State<AdminPassword>>()?;

        // Validate our user.
        if auth.0.username() == "falconeri" && auth.0.password() == password.0 {
            Outcome::Success(User)
        } else {
            Outcome::Failure((Status::Unauthorized, ()))
        }
    }
}

/// Extract HTTP Basic Auth credentials from a request.
fn basic_auth_from_request(
    request: &Request<'_>,
) -> Result<Option<Authorization<Basic>>> {
    // Extract our `Authorization` headers as a `Vec<HeaderValue>`.
    let auth_headers = request
        .headers()
        .get(Authorization::<Basic>::name().as_str())
        .map(|s| HeaderValue::from_str(s))
        .collect::<result::Result<Vec<HeaderValue>, _>>()?;

    if auth_headers.is_empty() {
        Ok(None)
    } else {
        let auth = Authorization::<Basic>::decode(&mut auth_headers.iter())
            .map_err(|_| format_err!("expected Authorization Basic header"))?;
        Ok(Some(auth))
    }
}
