//! How should we connect to PostgreSQL and `falconerid`?

use std::thread;
use std::time::Duration;

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
    #[tracing::instrument(level = "trace")]
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
    /// failure.
    #[tracing::instrument(skip(f), level = "trace")]
    pub fn retry_if_appropriate<F, T>(self, mut f: F) -> Result<T>
    where
        F: FnMut() -> Result<T>,
    {
        if !self.should_retry_by_default() {
            return f();
        }

        const MAX_RETRIES: u32 = 10;
        const INITIAL_INTERVAL: Duration = Duration::from_millis(500);
        const MAX_INTERVAL: Duration = Duration::from_secs(60);

        let mut interval = INITIAL_INTERVAL;
        let mut last_err = None;

        for _ in 0..MAX_RETRIES {
            match f() {
                Ok(value) => return Ok(value),
                Err(err) => {
                    error!("retrying after error: {}", err);
                    last_err = Some(err);
                    thread::sleep(interval);
                    interval = (interval * 2).min(MAX_INTERVAL);
                }
            }
        }

        Err(last_err.unwrap())
    }
}
