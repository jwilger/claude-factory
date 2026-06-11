//! Routing logic — pure functions for resolving work types to executors.

use crate::types::routing::{ExecutorSpec, RoutingTable, WorkType};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RoutingError {
    #[error("no routing entry found for work type {work_type:?}")]
    NoEntry { work_type: WorkType },
}

/// Resolve the executor for a work type, or return an error.
///
/// # Errors
/// Returns `RoutingError::NoEntry` if no entry exists for `work_type`.
pub fn resolve(table: &RoutingTable, work_type: WorkType) -> Result<&ExecutorSpec, RoutingError> {
    table
        .resolve(work_type)
        .ok_or(RoutingError::NoEntry { work_type })
}
