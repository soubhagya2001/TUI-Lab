//! Step supervision: bound any future with a timeout (docs/02 §2.2).
//!
//! Parallel session fan-out (`JoinSet`) arrives with the Phase 3 runner,
//! which is its first real consumer.

use std::future::Future;
use std::time::Duration;

use crate::error::{Result, RuntimeError};

/// Run `future` to completion or fail with [`RuntimeError::Timeout`].
pub async fn run_with_timeout<F, T>(future: F, timeout: Duration, what: &str) -> Result<T>
where
    F: Future<Output = T>,
{
    tokio::time::timeout(timeout, future)
        .await
        .map_err(|_| RuntimeError::Timeout(format!("{what} exceeded {timeout:?}")))
}
