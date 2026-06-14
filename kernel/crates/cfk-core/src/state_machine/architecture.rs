//! Architecture phase state machine — pure types.
//!
//! Each architecture work item represents one ADR proposal. The work item
//! progresses from drafting through a reviewer gate to accepted or rejected.

use crate::types::ids::AdrId;
use serde::{Deserialize, Serialize};

/// Phases of an ADR work item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdrPhase {
    /// Architect agent is drafting the ADR.
    Drafting,
    /// Draft submitted; reviewer agent is checking for conflicts.
    PendingReview,
    /// Reviewer approved; ADR is accepted into the registry.
    Accepted,
    /// Reviewer vetoed the ADR; a human must decide next steps.
    PendingHumanDecision,
}

/// Runtime state for an in-progress architecture (ADR) work item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdrWorkItemState {
    pub phase: AdrPhase,
    /// Assigned when the draft is submitted.
    pub adr_id: Option<AdrId>,
    pub title: Option<String>,
    pub content: Option<String>,
}

impl AdrWorkItemState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            phase: AdrPhase::Drafting,
            adr_id: None,
            title: None,
            content: None,
        }
    }
}

impl Default for AdrWorkItemState {
    fn default() -> Self {
        Self::new()
    }
}
