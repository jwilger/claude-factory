//! Lease types — tokens that grant exclusive access to a work item.

use crate::types::ids::{LeaseId, WorkItemId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A token granting a session exclusive access to a work item.
/// Sessions must hold a valid lease before writing product source code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lease {
    pub id: LeaseId,
    pub work_item_id: WorkItemId,
    pub session_identity: SessionIdentity,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Identifies the Claude Code session holding a lease.
/// In v1 this is a human-readable string (hostname + pid or similar).
/// Future: tie to cryptographic session identity.
#[nutype::nutype(
    sanitize(trim),
    validate(not_empty),
    derive(Debug, Display, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)
)]
pub struct SessionIdentity(String);

impl Lease {
    #[must_use]
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.expires_at.is_some_and(|exp| now > exp)
    }
}
