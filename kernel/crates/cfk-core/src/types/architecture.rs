//! Architecture phase types — ADR records and status.

use crate::types::ids::{AdrId, WorkItemId};
use serde::{Deserialize, Serialize};

/// Status of an Architecture Decision Record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdrStatus {
    /// ADR has been drafted; under review.
    Proposed,
    /// ADR was accepted by the reviewer gate.
    Accepted,
    /// ADR was rejected by the reviewer gate.
    Rejected,
    /// ADR was superseded by a newer ADR.
    Superseded(AdrId),
}

/// A fully-accepted Architecture Decision Record, stored in the global registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdrRecord {
    pub id: AdrId,
    pub work_item_id: WorkItemId,
    pub title: String,
    pub content: String,
    pub status: AdrStatus,
}
