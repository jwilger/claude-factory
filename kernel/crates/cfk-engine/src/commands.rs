//! Command handling — the imperative shell that drives `cfk-core` state machines.
//!
//! Each `handle_*` function: reads state from the project projection, calls
//! `cfk-core` pure functions, and writes the result back.
//! No business logic lives here.

use crate::project::ProjectState;
use cfk_core::{
    state_machine::work_item::{next_ready, validate_claim},
    types::{
        ids::{LeaseId, StepId},
        lease::{Lease, SessionIdentity},
        routing::WorkType,
        step::{IdleReason, ReadyStep, StepAction, StepPrompt},
    },
};
use chrono::Utc;
use serde::Serialize;
use thiserror::Error;

/// Errors from command handling.
#[derive(Debug, Error)]
pub enum CommandError {
    #[error("no routing entry for work type {0:?}")]
    NoRouting(WorkType),

    #[error("work item error: {0}")]
    WorkItem(#[from] cfk_core::state_machine::work_item::WorkItemError),

    #[error("session identity is required")]
    MissingSessionIdentity,
}

/// Response from `cf_next_step`.
#[derive(Debug, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum NextStepResponse {
    Ready(ReadyStep),
    Idle(IdleReason),
}

/// Handle `cf_next_step` — find the next ready work item and build the step.
///
/// # Errors
/// Returns `CommandError::NoRouting` if the ready item's work type has no executor.
///
/// # Panics
/// Panics only if the internal fallback prompt literal is somehow empty, which
/// cannot happen in practice.
pub fn handle_next_step(
    state: &ProjectState,
    phase_filter: Option<cfk_core::types::phase::PhaseKind>,
) -> Result<NextStepResponse, CommandError> {
    let items: Vec<_> = state
        .work_items
        .iter()
        .filter(|i| phase_filter.is_none_or(|p| i.phase == p))
        .cloned()
        .collect();

    let Some(item) = next_ready(&items) else {
        return Ok(NextStepResponse::Idle(IdleReason::NoReadyWork));
    };

    let executor = cfk_core::routing::resolve(&state.routing, item.work_type)
        .map_err(|_| CommandError::NoRouting(item.work_type))?
        .clone();

    let prompt = StepPrompt::try_new(item.description.clone()).unwrap_or_else(|_| {
        // Fallback for malformed descriptions; literal is non-empty so cannot fail.
        StepPrompt::try_new("Process this work item.".to_string())
            .expect("fallback prompt literal is non-empty")
    });

    let step_id = StepId::new();

    let step = ReadyStep {
        step_id,
        work_item_id: item.id.clone(),
        phase: item.phase,
        action: StepAction::SpawnAgent {
            executor,
            prompt,
            output_schema: None,
        },
    };

    Ok(NextStepResponse::Ready(step))
}

/// Handle `cf_claim` — validate and create a lease for a work item.
///
/// # Errors
/// Returns errors if the item cannot be claimed or session identity is missing.
pub fn handle_claim(
    state: &ProjectState,
    work_item_id: &cfk_core::types::ids::WorkItemId,
    session_identity: &str,
) -> Result<Lease, CommandError> {
    let item = state
        .work_items
        .iter()
        .find(|i| &i.id == work_item_id)
        .ok_or_else(|| CommandError::WorkItem(
            cfk_core::state_machine::work_item::WorkItemError::NotClaimable {
                id: work_item_id.clone(),
                status: cfk_core::state_machine::work_item::WorkItemStatus::Abandoned,
            },
        ))?;

    validate_claim(item)?;

    let identity = SessionIdentity::try_new(session_identity.to_string())
        .map_err(|_| CommandError::MissingSessionIdentity)?;

    Ok(Lease {
        id: LeaseId::new(),
        work_item_id: work_item_id.clone(),
        session_identity: identity,
        granted_at: Utc::now(),
        expires_at: None,
    })
}
