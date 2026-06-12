//! Work item state machine.
//!
//! A work item is the kernel's fundamental unit of trackable work.
//! It can be in one of a small set of states; transitions are validated here.

use crate::types::{
    ids::{LeaseId, StepId, WorkItemId},
    phase::PhaseKind,
    routing::WorkType,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemStatus {
    /// Ready to be claimed and worked on.
    Ready,
    /// Claimed; a lease exists.
    InProgress,
    /// Waiting on a dependency (another work item or a human decision).
    Blocked,
    /// Completed successfully.
    Done,
    /// Abandoned (e.g. superseded by a new work item).
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    pub id: WorkItemId,
    pub phase: PhaseKind,
    pub work_type: WorkType,
    pub status: WorkItemStatus,
    pub description: String,
    pub active_lease: Option<LeaseId>,
    pub active_step: Option<StepId>,
    /// The emc slice slug this work item was ingested from, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emc_slug: Option<String>,
}

impl WorkItem {
    #[must_use]
    pub fn new(
        id: WorkItemId,
        phase: PhaseKind,
        work_type: WorkType,
        description: String,
    ) -> Self {
        Self {
            id,
            phase,
            work_type,
            status: WorkItemStatus::Ready,
            description,
            active_lease: None,
            active_step: None,
            emc_slug: None,
        }
    }

    /// Create a work item sourced from an emc slice.
    #[must_use]
    pub fn from_emc_slice(
        id: WorkItemId,
        phase: PhaseKind,
        work_type: WorkType,
        description: String,
        emc_slug: String,
    ) -> Self {
        Self {
            id,
            phase,
            work_type,
            status: WorkItemStatus::Ready,
            description,
            active_lease: None,
            active_step: None,
            emc_slug: Some(emc_slug),
        }
    }

    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.status == WorkItemStatus::Ready
    }
}

/// Errors that can occur in work item state transitions.
#[derive(Debug, Error)]
pub enum WorkItemError {
    #[error("work item {id:?} is not in a claimable state (current: {status:?})")]
    NotClaimable {
        id: WorkItemId,
        status: WorkItemStatus,
    },

    #[error("work item {id:?} has no active step to submit")]
    NoActiveStep { id: WorkItemId },
}

/// State produced by claiming a work item (pure; no I/O).
#[derive(Debug)]
pub struct WorkItemClaimed {
    pub work_item_id: WorkItemId,
    pub lease_id: LeaseId,
    pub step_id: StepId,
}

/// Validate a claim operation without mutating state.
///
/// # Errors
/// Returns `WorkItemError::NotClaimable` if the item is not in `Ready` status.
pub fn validate_claim(item: &WorkItem) -> Result<(), WorkItemError> {
    if item.status != WorkItemStatus::Ready {
        return Err(WorkItemError::NotClaimable {
            id: item.id.clone(),
            status: item.status,
        });
    }
    Ok(())
}

/// Determine the next ready work item from a list, in order.
/// Returns the first item whose status is `Ready`.
#[must_use]
pub fn next_ready(items: &[WorkItem]) -> Option<&WorkItem> {
    items.iter().find(|i| i.is_ready())
}

/// State from completing a step (pure; no I/O).
#[derive(Debug)]
pub struct WorkItemState {
    pub item: WorkItem,
}

impl WorkItemState {
    #[must_use]
    pub fn new(item: WorkItem) -> Self {
        Self { item }
    }
}
