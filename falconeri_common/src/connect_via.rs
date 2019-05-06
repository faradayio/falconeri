//! How should we connect to PostgreSQL and `falconerid`?

use backoff::{self, ExponentialBackoff, Operation};
use std::result;

use crate::prelude::*;

/// How should we connect to the database?
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectVia {
    /// Assume we're connecting via a `kubectl proxy`.
    Proxy,
    /// Assume we're connecting via internal cluster networking and DNS.
    Cluster,
}

impl ConnectVia {
    /// Should we retry failed connections?
    pub fn should_retry_by_default(self) -> bool {
        match self {
            // When we're connected via a proxy from outside the cluster, it's
            // generally better to just pass errors straight through
            // immediately.
            ConnectVia::Proxy => false,
            // When we're running on the cluster, we want to retry network
            // operations by default, because:
            //
            // 1. Kubernetes cluster DNS is extremely flaky, and
            // 2. Cluster operations may involve 1000+ worker-hours. At this
            //    scale, something will inevitably break.
            ConnectVia::Cluster => true,
        }
    }

    /// Run the function `f`. If `self.should_retry_by_default()` is true, retry
    /// failures using exponential backoff. Return either the result or the final
    /// final failure.
    pub fn retry_if_appropriate<F, T>(self, mut f: F) -> Result<T>
    where
        F: FnMut() -> Result<T>,
    {
        // Wrap `f` up into an operation that results am appropriate
        // `backoff::Error` on failure.
        let mut operation = || -> result::Result<T, backoff::Error<Error>> {
            f().map_err(|err| {
                if self.should_retry_by_default() {
                    backoff::Error::Transient(err)
                } else {
                    backoff::Error::Permanent(err)
                }
            })
        };

        // Specify what kind of backoff to use.
        let mut backoff = ExponentialBackoff::default();

        // Run our operation, retrying if necessary.
        let value = operation
            .retry(&mut backoff)
            // Unwrap the backoff error into something we can handle. This should
            // have been built in.
            .map_err(|e| match e {
                backoff::Error::Transient(e) => e,
                backoff::Error::Permanent(e) => e,
            })?;
        Ok(value)
    }
}
