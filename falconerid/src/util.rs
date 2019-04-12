//! Various Rocket-related utilities.

use falconeri_common::prelude::*;
use rocket::{http::RawStr, request::FromParam};
use std::result;

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
