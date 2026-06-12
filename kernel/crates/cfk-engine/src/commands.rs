//! Command handling — the imperative shell that drives `cfk-core` state machines.
//!
//! Each `handle_*` function: reads state from the project projection, calls
//! `cfk-core` pure functions, and writes the result back.
//! No business logic lives here.

use crate::project::ProjectState;
use cfk_core::{
    routing::RoutingError,
    state_machine::work_item::{next_ready, validate_claim},
    types::{
        gate::{GateKind, GateVerdict},
        ids::{LeaseId, StepId, WorkItemId},
        lease::{Lease, SessionIdentity},
        metrics::{MetricsSummary, StepOutcome, WorkTypeMetricEntry},
        phase::PhaseKind,
        step::{IdleReason, ReadyStep},
        tdd::TddPhase,
    },
};
use chrono::Utc;
use serde::Serialize;
use thiserror::Error;

/// Errors from command handling.
#[derive(Debug, Error)]
pub enum CommandError {
    #[error("routing configuration error: {0}")]
    Routing(#[from] RoutingError),

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

    #[error("work item {0:?} is not in progress")]
    NotInProgress(WorkItemId),

    #[error("triage context not found for work item {0:?}")]
    TriageContextNotFound(WorkItemId),

    #[error("invalid submission input: {0}")]
    InvalidInput(String),
}

impl From<cfk_core::steps::StepError> for CommandError {
    fn from(e: cfk_core::steps::StepError) -> Self {
        match e {
            cfk_core::steps::StepError::Routing(r) => CommandError::Routing(r),
        }
    }
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
/// Returns `Ok(None)` if the item has no dev state or the phase is terminal.
fn tdd_step(
    state: &ProjectState,
    work_item_id: &WorkItemId,
) -> Result<Option<ReadyStep>, CommandError> {
    let Some(dev) = state.dev_states.get(work_item_id) else {
        return Ok(None);
    };
    let Some(frame) = dev.current_frame() else {
        return Ok(None);
    };
    let Some(item) = state.work_items.iter().find(|i| &i.id == work_item_id) else {
        return Ok(None);
    };

    let Some(action) = cfk_core::steps::tdd_step_action(frame, &item.description, &state.routing)?
    else {
        return Ok(None);
    };

    Ok(Some(ReadyStep {
        step_id: StepId::new(),
        work_item_id: work_item_id.clone(),
        phase: item.phase,
        action,
    }))
}

/// Build the next step for a review work item based on its current phase.
///
/// Returns `Ok(None)` for the terminal `Merged` phase or if no pending triage
/// item can be resolved when `CommentTriagePending` is active.
fn review_step(
    state: &ProjectState,
    work_item_id: &WorkItemId,
) -> Result<Option<ReadyStep>, CommandError> {
    let Some(item) = state.work_items.iter().find(|i| &i.id == work_item_id) else {
        return Ok(None);
    };
    let review = state.review_states.get(work_item_id);

    let pending_triage = review
        .and_then(|r| r.pending_triage.first())
        .and_then(|(cid, tid)| {
            state
                .work_items
                .iter()
                .find(|i| &i.id == tid)
                .map(|ti| (cid.as_str(), ti.description.as_str()))
        });

    let Some(action) = cfk_core::steps::review_step_action(
        review,
        &item.description,
        pending_triage,
        &state.routing,
    )?
    else {
        return Ok(None);
    };

    Ok(Some(ReadyStep {
        step_id: StepId::new(),
        work_item_id: work_item_id.clone(),
        phase: item.phase,
        action,
    }))
}

/// Build the next step for a discovery work item based on its current phase.
///
/// Returns `Ok(None)` for the terminal `Approved` phase.
fn discovery_step(
    state: &ProjectState,
    work_item_id: &WorkItemId,
) -> Result<Option<ReadyStep>, CommandError> {
    let Some(item) = state.work_items.iter().find(|i| &i.id == work_item_id) else {
        return Ok(None);
    };
    let disc = state.discovery_states.get(work_item_id);

    let Some(action) =
        cfk_core::steps::discovery_step_action(disc, &item.description, &state.routing)?
    else {
        return Ok(None);
    };

    Ok(Some(ReadyStep {
        step_id: StepId::new(),
        work_item_id: work_item_id.clone(),
        phase: item.phase,
        action,
    }))
}

/// Build the next step for an architecture (ADR) work item based on its current phase.
///
/// Returns `Ok(None)` for terminal phases (`Accepted`, `Rejected`).
fn architecture_step(
    state: &ProjectState,
    work_item_id: &WorkItemId,
) -> Result<Option<ReadyStep>, CommandError> {
    let Some(item) = state.work_items.iter().find(|i| &i.id == work_item_id) else {
        return Ok(None);
    };
    let adr = state.adr_states.get(work_item_id);

    let Some(action) = cfk_core::steps::architecture_step_action(
        adr,
        &item.description,
        &state.adrs,
        &state.routing,
    )?
    else {
        return Ok(None);
    };

    Ok(Some(ReadyStep {
        step_id: StepId::new(),
        work_item_id: work_item_id.clone(),
        phase: item.phase,
        action,
    }))
}

/// Build the next step for a design-system work item based on its current phase.
///
/// Returns `Ok(None)` for the terminal `Done` phase.
fn design_step(
    state: &ProjectState,
    work_item_id: &WorkItemId,
) -> Result<Option<ReadyStep>, CommandError> {
    let Some(item) = state.work_items.iter().find(|i| &i.id == work_item_id) else {
        return Ok(None);
    };
    let ds = state.design_states.get(work_item_id);

    let Some(action) = cfk_core::steps::design_step_action(
        ds,
        &item.description,
        &state.design_inventory,
        &state.routing,
    )?
    else {
        return Ok(None);
    };

    Ok(Some(ReadyStep {
        step_id: StepId::new(),
        work_item_id: work_item_id.clone(),
        phase: item.phase,
        action,
    }))
}

/// Handle `cf_next_step` — find the next ready work item and build the step.
///
/// For development-phase items already in progress, returns the TDD-phase-
/// specific step. For other phases (or items not yet in TDD), returns the
/// general executor step based on the routing table.
///
/// # Errors
/// Returns `CommandError::Routing` if the ready item's work type has no
/// executor in the routing table — a misconfiguration that must be surfaced
/// rather than silently treated as "no work ready".
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
        if let Some(dev) = state.dev_states.get(&item.id) {
            if dev.current_phase() == Some(&TddPhase::Done) {
                continue;
            }
            if let Some(step) = tdd_step(state, &item.id)? {
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
        if let Some(step) = review_step(state, &item.id)? {
            return Ok(NextStepResponse::Ready(step));
        }
    }

    // 1c. Check in-progress discovery items.
    for item in &state.work_items {
        if phase_filter.is_some_and(|p| item.phase != p) {
            continue;
        }
        if item.status != cfk_core::state_machine::work_item::WorkItemStatus::InProgress {
            continue;
        }
        if item.phase != PhaseKind::Discovery {
            continue;
        }
        if let Some(step) = discovery_step(state, &item.id)? {
            return Ok(NextStepResponse::Ready(step));
        }
    }

    // 1d. Check in-progress architecture items.
    for item in &state.work_items {
        if phase_filter.is_some_and(|p| item.phase != p) {
            continue;
        }
        if item.status != cfk_core::state_machine::work_item::WorkItemStatus::InProgress {
            continue;
        }
        if item.phase != PhaseKind::Architecture {
            continue;
        }
        if let Some(step) = architecture_step(state, &item.id)? {
            return Ok(NextStepResponse::Ready(step));
        }
    }

    // 1e. Check in-progress design-system items.
    for item in &state.work_items {
        if phase_filter.is_some_and(|p| item.phase != p) {
            continue;
        }
        if item.status != cfk_core::state_machine::work_item::WorkItemStatus::InProgress {
            continue;
        }
        if item.phase != PhaseKind::DesignSystem {
            continue;
        }
        if let Some(step) = design_step(state, &item.id)? {
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

    let executor = cfk_core::routing::resolve(&state.routing, item.work_type)?.clone();

    let prompt = cfk_core::prompts::generic_step(&item.description);
    let step_id = StepId::new();

    let step = ReadyStep {
        step_id,
        work_item_id: item.id.clone(),
        phase: item.phase,
        action: cfk_core::types::step::StepAction::SpawnAgent {
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

/// Return a summary of per-work-type metrics from the current projection.
///
/// The summary is sorted by veto rate descending so the highest-veto routes
/// appear first — those are the candidates for routing table tuning.
#[must_use]
pub fn handle_metrics(state: &ProjectState) -> MetricsSummary {
    let mut entries: Vec<WorkTypeMetricEntry> = state
        .metrics
        .iter()
        .map(|(work_type, m)| WorkTypeMetricEntry {
            work_type: *work_type,
            approvals: m.approvals,
            vetoes: m.vetoes,
            completions: m.completions,
            veto_rate: m.veto_rate(),
            avg_tokens: m.avg_tokens(),
        })
        .collect();

    entries.sort_by(|a, b| {
        b.veto_rate
            .unwrap_or(0.0)
            .partial_cmp(&a.veto_rate.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    MetricsSummary { entries }
}

/// Build a `StepOutcomeRecorded` event for the given work type and outcome.
///
/// # Errors
/// Returns `CommandError::NotFound` if the work item does not exist.
pub fn handle_record_outcome(
    state: &ProjectState,
    work_item_id: &WorkItemId,
    outcome: StepOutcome,
    tokens_used: Option<u32>,
) -> Result<crate::events::FactoryEvent, CommandError> {
    let item = state
        .work_items
        .iter()
        .find(|i| &i.id == work_item_id)
        .ok_or_else(|| CommandError::NotFound(work_item_id.clone()))?;

    Ok(crate::events::FactoryEvent::StepOutcomeRecorded {
        work_type: item.work_type,
        outcome,
        tokens_used,
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
    if let Some(lease) = state.leases.iter().find(|l| &l.work_item_id == work_item_id)
        && let Ok(reviewer_identity) =
            cfk_core::types::lease::SessionIdentity::try_new(reviewer_id.to_string())
        && lease.session_identity == reviewer_identity
    {
        return Err(CommandError::ReviewerIsAuthor);
    }
    Ok(())
}

// ── Submission ────────────────────────────────────────────────────────────────

/// The payload for a `cf_submit` call, resolved at the MCP JSON boundary.
pub enum SubmissionPayload {
    /// Test code submitted during the `WriteTest` TDD phase.
    Test {
        test_content: cfk_core::types::tdd::TestCode,
    },
    /// Implementation result; optionally with a drill-down into a narrower unit test.
    Implementation {
        drill_down: Option<cfk_core::types::tdd::DrillDownDescription>,
    },
    /// A reply to a PR comment from a triage work item.
    TriageReply {
        reply: cfk_core::types::forge::CommentBody,
    },
    /// Generic submission evidence for non-TDD phases.
    Generic(serde_json::Value),
}

/// The outcome returned from `handle_submission` for the MCP layer to act on.
pub enum SubmissionOutcome {
    /// The TDD state machine advanced to this phase.
    AdvancedTo(TddPhase),
    /// A drill-down sub-frame was pushed; item re-enters `WriteTest` at greater depth.
    DrillDownPushed { depth: u32 },
    /// Submission acknowledged; item completed or gate-pending.
    Acknowledged,
    /// A triage reply must be posted to the forge before the caller appends events.
    CommentQueued { pr_number: u64, reply: String },
}

/// Handle a `cf_submit` call — emit the appropriate events for the current TDD
/// phase or work item type.
///
/// Returns the events to append and an outcome the caller uses to perform any
/// required forge effects (e.g. posting a PR comment for triage items).
///
/// # Errors
/// Returns `CommandError` if the work item is missing, not in progress, or the
/// triage context cannot be resolved.
pub fn handle_submission(
    state: &ProjectState,
    work_item_id: WorkItemId,
    payload: SubmissionPayload,
) -> Result<(Vec<crate::events::FactoryEvent>, SubmissionOutcome), CommandError> {
    use cfk_core::state_machine::work_item::WorkItemStatus;
    use crate::events::FactoryEvent;

    let item_status = state
        .work_items
        .iter()
        .find(|i| i.id == work_item_id)
        .map(|i| i.status)
        .ok_or_else(|| CommandError::NotFound(work_item_id.clone()))?;

    let current_tdd_phase = state
        .dev_states
        .get(&work_item_id)
        .and_then(|d| d.current_phase())
        .cloned();

    let frame_depth = state
        .dev_states
        .get(&work_item_id)
        .and_then(|d| d.current_frame())
        .map_or(0, |f| f.depth);

    let author_str = state
        .leases
        .iter()
        .find(|l| l.work_item_id == work_item_id)
        .map_or_else(|| "unknown".to_string(), |l| l.session_identity.to_string());

    match (current_tdd_phase, payload) {
        (Some(TddPhase::WriteTest), SubmissionPayload::Test { test_content }) => {
            let author_identity = cfk_core::types::tdd::AuthorIdentity::try_new(author_str.clone())
                .map_err(|_| CommandError::InvalidInput(format!("invalid author identity: {author_str}")))?;
            let events = vec![FactoryEvent::TddTestSubmitted {
                work_item_id,
                frame_depth,
                test_content,
                author_identity,
            }];
            Ok((events, SubmissionOutcome::AdvancedTo(TddPhase::TestReviewGate)))
        }

        (Some(TddPhase::Implement), SubmissionPayload::Implementation { drill_down: Some(child_description) }) => {
            let child_depth = frame_depth + 1;
            let events = vec![FactoryEvent::TddDrillDownPushed {
                work_item_id,
                child_description,
                child_depth,
            }];
            Ok((events, SubmissionOutcome::DrillDownPushed { depth: child_depth }))
        }

        (Some(TddPhase::Implement), SubmissionPayload::Implementation { drill_down: None }) => {
            let events = vec![FactoryEvent::TddPhaseAdvanced {
                work_item_id,
                frame_depth,
                new_phase: TddPhase::CheckProgress,
            }];
            Ok((events, SubmissionOutcome::Acknowledged))
        }

        (Some(_), _) => {
            // Gate phase or other — acknowledge without advancing
            Ok((vec![], SubmissionOutcome::Acknowledged))
        }

        (None, SubmissionPayload::TriageReply { reply }) => {
            if item_status != WorkItemStatus::InProgress {
                return Err(CommandError::NotInProgress(work_item_id));
            }

            let triage_info = state.review_states.iter().find_map(|(review_wid, rs)| {
                rs.pending_triage
                    .iter()
                    .find(|(_, tid)| tid == &work_item_id)
                    .map(|(cid, _)| (review_wid.clone(), cid.clone(), rs.pr_number))
            });

            let (review_wid, cid_raw, pr_number_opt) = triage_info
                .ok_or_else(|| CommandError::TriageContextNotFound(work_item_id.clone()))?;

            let pr_number = pr_number_opt.unwrap_or(0);
            let reply_str = reply.into_inner();

            let comment_id = cfk_core::types::forge::CommentId::try_new(cid_raw.clone())
                .map_err(|_| CommandError::InvalidInput(format!("invalid comment id: {cid_raw}")))?;

            let events = vec![
                FactoryEvent::ReviewCommentPosted {
                    review_work_item_id: review_wid,
                    comment_id,
                    triage_item_id: work_item_id.clone(),
                },
                FactoryEvent::WorkItemCompleted { work_item_id },
            ];

            Ok((events, SubmissionOutcome::CommentQueued { pr_number, reply: reply_str }))
        }

        (None, _) => {
            if item_status != WorkItemStatus::InProgress {
                return Err(CommandError::NotInProgress(work_item_id));
            }
            let events = vec![FactoryEvent::WorkItemCompleted { work_item_id }];
            Ok((events, SubmissionOutcome::Acknowledged))
        }
    }
}
