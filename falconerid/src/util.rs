//! Various Rocket-related utilities.

use falconeri_common::{db, prelude::*};
use headers::{authorization::Basic, Authorization, Header, HeaderValue};
use rocket::{
    self, fairing,
    http::Status,
    request::{self, FromRequest, Outcome, Request},
    response::{self, Responder, Response},
    State,
};
use std::{io, ops, result};

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
        fairing::AdHoc::try_on_ignite("DbConn", |rocket| {
            Box::pin(async move {
                #[derive(Deserialize)]
                struct Config {
                    workers: u32,
                }
                let config = rocket
                    .figment()
                    .extract::<Config>()
                    .expect("we should always have a config with `workers` set");

                match db::pool(config.workers, ConnectVia::Cluster) {
                    Ok(pool) => Ok(rocket.manage(DbPool(pool))),
                    Err(err) => {
                        error!("failed to initialize database pool");
                        error!("{:?}", err);
                        Err(rocket)
                    }
                }
            })
        })
    }
}

// Rocket uses this to fetch `DbConn` parameters from the HTTP request
// automatically.
#[rocket::async_trait]
impl<'r> FromRequest<'r> for DbConn {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, ()> {
        // Try to get the connection pool attached to our server.
        let pool = match request.guard::<&State<DbPool>>().await {
            Outcome::Success(pool) => pool,
            Outcome::Failure(failure) => return Outcome::Failure(failure),
            Outcome::Forward(forward) => return Outcome::Forward(forward),
        };

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
        fairing::AdHoc::try_on_ignite("User", |rocket| {
            Box::pin(async move {
                match db::postgres_password(ConnectVia::Cluster) {
                    Ok(password) => Ok(rocket.manage(AdminPassword(password))),
                    Err(err) => {
                        error!("failed to look up admin password");
                        error!("{:?}", err);
                        Err(rocket)
                    }
                }
            })
        })
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, ()> {
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
        let password = match request.guard::<&State<AdminPassword>>().await {
            Outcome::Success(password) => password,
            Outcome::Failure(failure) => return Outcome::Failure(failure),
            Outcome::Forward(forward) => return Outcome::Forward(forward),
        };

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

/// An error type for `falconerid`. Ideally, this should be an enum with members
/// like `NotFound` and `Other`, which would allow us to send 404 responses,
/// etc. But for now it's just a wrapper.
#[derive(Debug)]
pub struct FalconeridError(Error);

impl<'r, 'o: 'r> Responder<'r, 'o> for FalconeridError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        // Log our full error, including the backtrace.
        error!("{}", self.0.display_causes_without_backtrace());

        // Put the error message in the payload for now. This might become JSON
        // in the future.
        let payload = format!("{}", self.0.display_causes_without_backtrace());
        Response::build()
            .sized_body(payload.len(), io::Cursor::new(payload))
            .header(rocket::http::ContentType::Plain)
            .status(Status::InternalServerError)
            .ok()
    }
}

impl From<Error> for FalconeridError {
    fn from(err: Error) -> Self {
        FalconeridError(err)
    }
}

/// The result type of `falconerid` handler.
pub type FalconeridResult<T> = result::Result<T, FalconeridError>;
