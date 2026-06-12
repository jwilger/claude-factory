//! Command handling — the imperative shell that drives `cfk-core` state machines.
//!
//! Each `handle_*` function: reads state from the project projection, calls
//! `cfk-core` pure functions, and writes the result back.
//! No business logic lives here.

use crate::project::ProjectState;
use cfk_core::{
    state_machine::{
        review::ReviewSlicePhase,
        work_item::{next_ready, validate_claim},
    },
    types::{
        gate::{GateKind, GateVerdict},
        ids::{LeaseId, StepId, WorkItemId},
        lease::{Lease, SessionIdentity},
        phase::PhaseKind,
        routing::WorkType,
        step::{IdleReason, ReadyStep, StepAction, StepPrompt},
        tdd::TddPhase,
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

    #[error("work item {0:?} not found")]
    NotFound(WorkItemId),

    #[error("work item {0:?} has no active TDD state")]
    NoTddState(WorkItemId),

    #[error("TDD error: {0}")]
    Tdd(#[from] cfk_core::state_machine::tdd::TddError),

    #[error("reviewer identity matches the author — a different session must review")]
    ReviewerIsAuthor,
}

/// Response from `cf_next_step`.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum NextStepResponse {
    Ready(ReadyStep),
    Idle(IdleReason),
}

/// Build the next step for a development work item based on its current TDD phase.
///
/// Returns `None` if the item has no dev state (should not happen for `InProgress` dev items).
fn tdd_step(state: &ProjectState, work_item_id: &WorkItemId) -> Option<ReadyStep> {
    let dev = state.dev_states.get(work_item_id)?;
    let frame = dev.current_frame()?;
    let item = state.work_items.iter().find(|i| &i.id == work_item_id)?;

    let step_id = StepId::new();

    let action: StepAction = match &frame.phase {
        TddPhase::WriteTest => {
            let prompt = build_prompt(&format!(
                "Write an outer behavioural test for: {}\n\nRequirements:\n\
                 - Test must be behavioural (tests what the system does, not how)\n\
                 - No mocks; use real I/O substitutes\n\
                 - Use semantic types in test code\n\
                 - Submit the complete test code in `test_content`.",
                item.description
            ));
            let exec = cfk_core::routing::resolve(
                &state.routing, WorkType::OuterBehavioralTestWriting,
            ).ok()?.clone();
            StepAction::SpawnAgent { executor: exec, prompt, output_schema: None }
        }
        TddPhase::TestReviewGate => {
            let test_content = frame.test_content.as_deref().unwrap_or("(no test content)");
            let prompt = build_prompt(&format!(
                "Review this test for the slice: {}\n\nTest code:\n```\n{test_content}\n```\n\n\
                 Checklist:\n\
                 - Is it behavioural (not implementation-coupled)?\n\
                 - Does it use no mocking libraries?\n\
                 - Does it use semantic types?\n\
                 - Will it fail for the right reason?\n\
                 Return verdict: approved or vetoed with reason.",
                item.description,
            ));
            let exec = cfk_core::routing::resolve(&state.routing, WorkType::TestReview)
                .ok()?.clone();
            StepAction::GateReview {
                gate_kind: GateKind::TestReview,
                executor: exec,
                prompt,
            }
        }
        TddPhase::RedCheck | TddPhase::CheckProgress => {
            let check = cfk_core::types::step::CheckName::try_new("tests".to_string())
                .expect("static literal is non-empty");
            StepAction::RunCheck { check_name: check }
        }
        TddPhase::Implement => {
            let first_error = frame.current_error.as_deref().unwrap_or("(unknown error)");
            let prompt = build_prompt(&format!(
                "Implement the narrowest change to address ONLY this error:\n\n\
                 ```\n{first_error}\n```\n\n\
                 Do NOT fix other errors. Do NOT refactor beyond what is required.\n\
                 If this error requires changes to more than one function boundary,\n\
                 set `drill_down_description` to describe the tighter unit test needed.\n\
                 Otherwise leave `drill_down_description` null.",
            ));
            let exec = cfk_core::routing::resolve(
                &state.routing, WorkType::NarrowestStepImplementation,
            ).ok()?.clone();
            StepAction::SpawnAgent { executor: exec, prompt, output_schema: None }
        }
        TddPhase::ImplReviewGate => {
            let prompt = build_prompt(&format!(
                "Review the implementation for the slice: {}\n\n\
                 Checklist:\n\
                 - Is this the narrowest possible change?\n\
                 - No mocking introduced?\n\
                 - Semantic types used throughout?\n\
                 - No unrelated refactoring?\n\
                 Return verdict: approved or vetoed with reason.",
                item.description,
            ));
            let exec = cfk_core::routing::resolve(
                &state.routing, WorkType::ImplementationReview,
            ).ok()?.clone();
            StepAction::GateReview {
                gate_kind: GateKind::ImplementationReview,
                executor: exec,
                prompt,
            }
        }
        TddPhase::LintCheck => {
            let check = cfk_core::types::step::CheckName::try_new("lint".to_string())
                .expect("static literal is non-empty");
            StepAction::RunCheck { check_name: check }
        }
        TddPhase::Done => return None, // handled as work item completion by the caller
    };

    Some(ReadyStep {
        step_id,
        work_item_id: work_item_id.clone(),
        phase: item.phase,
        action,
    })
}

/// Build the next step for a review work item based on its current `ReviewSlicePhase`.
///
/// Returns `None` if the phase is terminal (`Merged`).
fn review_step(state: &ProjectState, work_item_id: &WorkItemId) -> Option<ReadyStep> {
    let item = state.work_items.iter().find(|i| &i.id == work_item_id)?;
    let step_id = StepId::new();

    // No review state yet means the PR hasn't been opened — treat as WaitingForPr.
    let review = state.review_states.get(work_item_id);

    let action: StepAction = match review.map(|r| &r.phase) {
        None | Some(ReviewSlicePhase::WaitingForPr) => {
            let prompt = build_prompt(&format!(
                "Open a pull request for the slice: {}\n\n\
                 Provide a descriptive title and body. Submit via `cf_pr_open`.",
                item.description
            ));
            StepAction::OpenPr { prompt }
        }
        Some(ReviewSlicePhase::PrOpen) => StepAction::RunPrPoll,
        Some(ReviewSlicePhase::CommentTriagePending) => {
            let review = review.expect("Some checked above");
            // Find the first pending triage (comment_id, triage_item_id).
            let (comment_id, triage_item_id) = review.pending_triage.first()?;
            let triage_item = state.work_items.iter()
                .find(|i| &i.id == triage_item_id)?;
            let executor = cfk_core::routing::resolve(&state.routing, WorkType::PrCommentTriage)
                .ok()?.clone();
            let prompt = build_prompt(&format!(
                "Respond to this PR review comment for the slice: {}\n\n\
                 Comment ID: {comment_id}\n\
                 Comment: {}\n\n\
                 Write a concise, professional reply. Submit via `cf_submit`.",
                item.description,
                triage_item.description,
            ));
            StepAction::SpawnAgent { executor, prompt, output_schema: None }
        }
        Some(ReviewSlicePhase::AllGreen) => StepAction::MergePr,
        Some(ReviewSlicePhase::Merged) => return None,
    };

    Some(ReadyStep {
        step_id,
        work_item_id: work_item_id.clone(),
        phase: item.phase,
        action,
    })
}

fn build_prompt(text: &str) -> StepPrompt {
    StepPrompt::try_new(text.to_string()).unwrap_or_else(|_| {
        StepPrompt::try_new("Process this work item.".to_string())
            .expect("fallback literal is non-empty")
    })
}

/// Handle `cf_next_step` — find the next ready work item and build the step.
///
/// For development-phase items already in progress, returns the TDD-phase-
/// specific step. For other phases (or items not yet in TDD), returns the
/// general executor step based on the routing table.
///
/// # Errors
/// Returns `CommandError::NoRouting` if the ready item's work type has no executor.
///
/// # Panics
/// Panics only if an internal fallback prompt literal is somehow empty, which
/// cannot happen in practice.
pub fn handle_next_step(
    state: &ProjectState,
    phase_filter: Option<PhaseKind>,
) -> Result<NextStepResponse, CommandError> {
    // 1. Check if any in-progress development item needs a TDD step.
    for item in &state.work_items {
        if phase_filter.is_some_and(|p| item.phase != p) {
            continue;
        }
        if item.status != cfk_core::state_machine::work_item::WorkItemStatus::InProgress {
            continue;
        }
        if item.phase != PhaseKind::Development {
            continue;
        }
        // Development item in progress — look up its TDD state.
        if let Some(dev) = state.dev_states.get(&item.id) {
            if dev.current_phase() == Some(&TddPhase::Done) {
                // Slice is done; work item should be completed separately.
                continue;
            }
            if let Some(step) = tdd_step(state, &item.id) {
                return Ok(NextStepResponse::Ready(step));
            }
        }
    }

    // 1b. Check if any in-progress review item needs a review step.
    for item in &state.work_items {
        if phase_filter.is_some_and(|p| item.phase != p) {
            continue;
        }
        if item.status != cfk_core::state_machine::work_item::WorkItemStatus::InProgress {
            continue;
        }
        if item.phase != PhaseKind::Review {
            continue;
        }
        if let Some(step) = review_step(state, &item.id) {
            return Ok(NextStepResponse::Ready(step));
        }
    }

    // 2. Fall back to any ready item (including non-dev phases).
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

    let prompt = build_prompt(&item.description);
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
    work_item_id: &WorkItemId,
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

/// Validate a gate verdict submission (reviewer ≠ author check).
///
/// # Errors
/// Returns `CommandError::ReviewerIsAuthor` if the reviewer matches the work
/// item's claiming session identity.
pub fn validate_gate_verdict(
    state: &ProjectState,
    work_item_id: &WorkItemId,
    reviewer_id: &str,
    _gate_kind: GateKind,
    _verdict: &GateVerdict,
) -> Result<(), CommandError> {
    // Check reviewer != author by comparing with the claim lease.
    if let Some(lease) = state.leases.iter().find(|l| &l.work_item_id == work_item_id)
        && let Ok(reviewer_identity) =
            cfk_core::types::lease::SessionIdentity::try_new(reviewer_id.to_string())
        && lease.session_identity == reviewer_identity
    {
        return Err(CommandError::ReviewerIsAuthor);
    }
    Ok(())
}
