//! Discovery phase state machine — pure types.
//!
//! The discovery work item progresses through socratic dialogue, brief drafting,
//! and human approval before queuing workflows for event modeling.

use serde::{Deserialize, Serialize};

/// Phases of the discovery process for a single discovery work item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryPhase {
    /// Socratic discovery dialogue in progress.
    Dialogue,
    /// Agent has submitted a brief; awaiting human approval.
    BriefReady,
    /// Human approved the brief; workflows have been queued.
    Approved,
}

/// Runtime state for an in-progress discovery work item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryState {
    pub phase: DiscoveryPhase,
    /// The product brief drafted by the discovery agent.
    pub brief_content: Option<String>,
    /// Workflow names extracted from the brief (populated when brief is submitted).
    pub workflows: Vec<String>,
}

impl DiscoveryState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            phase: DiscoveryPhase::Dialogue,
            brief_content: None,
            workflows: Vec::new(),
        }
    }
}

impl Default for DiscoveryState {
    fn default() -> Self {
        Self::new()
    }
}
