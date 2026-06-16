//! Pure step-decision logic for all workflow phases.
//!
//! Each `*_step_action` function takes narrow inputs — no `ProjectState` —
//! and returns `Result<Option<StepAction>, StepError>`.
//!
//! - `Ok(None)` — phase is terminal; nothing to dispatch.
//! - `Ok(Some(action))` — a step is ready; the engine assembles the `ReadyStep`.
//! - `Err(StepError::Routing)` — routing table is misconfigured; must surface.

use crate::{
    prompts,
    routing::{resolve, RoutingError},
    state_machine::{
        architecture::AdrPhase,
        design::DesignPhase,
        discovery::DiscoveryPhase,
        review::{ReviewSlicePhase, ReviewSliceState},
    },
    types::{
        architecture::{AdrRecord, AdrStatus},
        design::DesignComponent,
        gate::GateKind,
        routing::{RoutingTable, WorkType},
        step::StepAction,
        tdd::{TddFrame, TddPhase},
    },
};
use thiserror::Error;

/// Errors from step-building logic.
#[derive(Debug, Error)]
pub enum StepError {
    #[error("routing configuration error: {0}")]
    Routing(#[from] RoutingError),
}

/// Determine the next `StepAction` for a TDD development frame.
///
/// Returns `Ok(None)` for the terminal `Done` phase.
///
/// # Errors
/// Returns `StepError::Routing` if the routing table has no entry for a
/// required work type.
pub fn tdd_step_action(
    frame: &TddFrame,
    description: &str,
    routing: &RoutingTable,
) -> Result<Option<StepAction>, StepError> {
    let action = match &frame.phase {
        TddPhase::WriteTest => {
            let executor = resolve(routing, WorkType::OuterBehavioralTestWriting)?.clone();
            let prompt = prompts::tdd_write_test(description);
            StepAction::SpawnAgent { executor, prompt, output_schema: None }
        }
        TddPhase::TestReviewGate => {
            let test_content = frame.test_content.as_ref()
                .map_or_else(|| "(no test content)".to_string(), ToString::to_string);
            let executor = resolve(routing, WorkType::TestReview)?.clone();
            let prompt = prompts::tdd_test_review(description, &test_content);
            StepAction::GateReview { gate_kind: GateKind::TestReview, executor, prompt }
        }
        TddPhase::RedCheck | TddPhase::CheckProgress => {
            StepAction::RunCheck { check_name: crate::types::step::well_known::TESTS.clone() }
        }
        TddPhase::Implement => {
            let first_error = frame.current_error.as_ref()
                .map_or_else(|| "(unknown error)".to_string(), ToString::to_string);
            let executor = resolve(routing, WorkType::NarrowestStepImplementation)?.clone();
            let prompt = prompts::tdd_implement(&first_error);
            StepAction::SpawnAgent { executor, prompt, output_schema: None }
        }
        TddPhase::ImplReviewGate => {
            let executor = resolve(routing, WorkType::ImplementationReview)?.clone();
            let prompt = prompts::tdd_impl_review(description);
            StepAction::GateReview { gate_kind: GateKind::ImplementationReview, executor, prompt }
        }
        TddPhase::LintCheck => {
            StepAction::RunCheck { check_name: crate::types::step::well_known::LINT.clone() }
        }
        TddPhase::Done => return Ok(None),
    };
    Ok(Some(action))
}

/// Determine the next `StepAction` for a review slice.
///
/// `pending_triage` provides `(comment_id, triage_description)` for the first
/// unresolved triage item, pre-resolved by the engine from the work-item list.
/// Returns `Ok(None)` for the terminal `Merged` phase or when
/// `CommentTriagePending` is active but no pending triage item could be found.
///
/// # Errors
/// Returns `StepError::Routing` if the routing table has no entry for
/// `PrCommentTriage`.
pub fn review_step_action(
    review: Option<&ReviewSliceState>,
    item_description: &str,
    pending_triage: Option<(&str, &str)>,
    routing: &RoutingTable,
) -> Result<Option<StepAction>, StepError> {
    let action = match review.map(|r| &r.phase) {
        None | Some(ReviewSlicePhase::WaitingForPr) => {
            let prompt = prompts::review_open_pr(item_description);
            StepAction::OpenPr { prompt }
        }
        Some(ReviewSlicePhase::PrOpen) => StepAction::RunPrPoll,
        Some(ReviewSlicePhase::CommentTriagePending) => {
            let Some((comment_id, triage_description)) = pending_triage else {
                return Ok(None);
            };
            let executor = resolve(routing, WorkType::PrCommentTriage)?.clone();
            let prompt = prompts::review_triage_comment(
                item_description,
                comment_id,
                triage_description,
            );
            StepAction::SpawnAgent { executor, prompt, output_schema: None }
        }
        Some(ReviewSlicePhase::AllGreen) => StepAction::MergePr,
        Some(ReviewSlicePhase::Merged) => return Ok(None),
    };
    Ok(Some(action))
}

/// Determine the next `StepAction` for a discovery work item.
///
/// Returns `Ok(None)` for the terminal `Approved` phase.
///
/// # Errors
/// Returns `StepError::Routing` if the routing table has no entry for
/// `SocraticDiscovery`.
pub fn discovery_step_action(
    disc: Option<&crate::state_machine::discovery::DiscoveryState>,
    description: &str,
    routing: &RoutingTable,
) -> Result<Option<StepAction>, StepError> {
    let action = match disc.map(|d| &d.phase) {
        None | Some(DiscoveryPhase::Dialogue) => {
            let executor = resolve(routing, WorkType::SocraticDiscovery)?.clone();
            let prompt = prompts::discovery_socratic(description);
            StepAction::SpawnAgent { executor, prompt, output_schema: None }
        }
        Some(DiscoveryPhase::BriefReady) => {
            let brief = disc
                .and_then(|d| d.brief_content.as_deref())
                .unwrap_or("(brief not available)");
            let question = prompts::discovery_brief_approval(description, brief);
            StepAction::AskHuman { question }
        }
        Some(DiscoveryPhase::Approved) => return Ok(None),
    };
    Ok(Some(action))
}

/// Determine the next `StepAction` for an architecture (ADR) work item.
///
/// `accepted_adrs` is the full global ADR registry; only accepted records are
/// used in the prompt.  Returns `Ok(None)` for terminal phases.
///
/// # Errors
/// Returns `StepError::Routing` if the routing table has no entry for
/// `AdrDrafting` or `AdrReview`.
pub fn architecture_step_action(
    adr: Option<&crate::state_machine::architecture::AdrWorkItemState>,
    description: &str,
    accepted_adrs: &[AdrRecord],
    routing: &RoutingTable,
) -> Result<Option<StepAction>, StepError> {
    let action = match adr.map(|a| &a.phase) {
        None | Some(AdrPhase::Drafting) => {
            let executor = resolve(routing, WorkType::AdrDrafting)?.clone();
            let accepted_lines: Vec<_> = accepted_adrs
                .iter()
                .filter(|r| r.status == AdrStatus::Accepted)
                .map(|r| format!("- {}: {}", r.title, r.content))
                .collect();
            let accepted_summary = if accepted_lines.is_empty() {
                "None yet.".to_string()
            } else {
                accepted_lines.join("\n")
            };
            let prompt = prompts::architecture_draft_adr(description, &accepted_summary);
            StepAction::SpawnAgent { executor, prompt, output_schema: None }
        }
        Some(AdrPhase::PendingReview) => {
            let content = adr
                .and_then(|a| a.content.as_deref())
                .unwrap_or("(content not available)");
            let title = adr
                .and_then(|a| a.title.as_deref())
                .unwrap_or("(untitled)");
            let executor = resolve(routing, WorkType::AdrReview)?.clone();
            let prompt = prompts::architecture_review_adr(title, content);
            StepAction::GateReview {
                gate_kind: GateKind::AdrReview,
                executor,
                prompt,
            }
        }
        Some(AdrPhase::PendingHumanDecision) => {
            let question = crate::types::step::HumanQuestion::from_static(
                "The ADR reviewer vetoed this draft. Do you want to: (1) requeue the architect \
                 to revise the ADR, (2) escalate the rejection for further review, or (3) abandon \
                 this ADR entirely?",
            );
            StepAction::AskHuman { question }
        }
        Some(AdrPhase::Accepted) => return Ok(None),
    };
    Ok(Some(action))
}

/// Determine the next `StepAction` for a design-system work item.
///
/// `design_inventory` is the current list of completed design components.
/// Returns `Ok(None)` for the terminal `Done` phase.
///
/// # Errors
/// Returns `StepError::Routing` if the routing table has no entry for
/// `DesignSystemBuild`.
pub fn design_step_action(
    ds: Option<&crate::state_machine::design::DesignWorkItemState>,
    description: &str,
    design_inventory: &[DesignComponent],
    routing: &RoutingTable,
) -> Result<Option<StepAction>, StepError> {
    let action = match ds.map(|d| &d.phase) {
        None | Some(DesignPhase::Building) => {
            let executor = resolve(routing, WorkType::DesignSystemBuild)?.clone();
            let inventory_lines: Vec<_> = design_inventory
                .iter()
                .map(|c| format!("- {} ({:?})", c.name, c.kind))
                .collect();
            let inventory_summary = if inventory_lines.is_empty() {
                "None yet.".to_string()
            } else {
                inventory_lines.join("\n")
            };
            let prompt = prompts::design_build_component(description, &inventory_summary);
            StepAction::SpawnAgent { executor, prompt, output_schema: None }
        }
        Some(DesignPhase::Done) => return Ok(None),
    };
    Ok(Some(action))
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test functions use expect/panic for assertion clarity"
)]
mod tests {
    use super::*;
    use crate::{
        types::{
            routing::{AgentName, ClaudeModel, ExecutorSpec, RoutingEntry, RoutingTable, WorkType},
            tdd::TddFrame,
        },
    };

    fn routing_with(entries: &[(WorkType, &str)]) -> RoutingTable {
        RoutingTable {
            entries: entries
                .iter()
                .map(|(wt, name)| RoutingEntry {
                    work_type: *wt,
                    executor: ExecutorSpec::Claude {
                        model: ClaudeModel::Sonnet,
                        #[expect(
                            clippy::expect_used,
                            reason = "test-only static literals; non-empty is guaranteed"
                        )]
                        agent_name: AgentName::try_new((*name).to_string())
                            .expect("valid agent name"),
                    },
                    notes: None,
                })
                .collect(),
        }
    }

    #[test]
    fn tdd_write_test_phase_returns_spawn_agent() {
        let routing = routing_with(&[(WorkType::OuterBehavioralTestWriting, "test-writer")]);
        let frame = TddFrame::new(0);
        let action = tdd_step_action(&frame, "login slice", &routing)
            .expect("step")
            .expect("Some action");
        assert!(matches!(action, StepAction::SpawnAgent { .. }));
    }

    #[test]
    fn tdd_done_phase_returns_none() {
        let routing = routing_with(&[]);
        let mut frame = TddFrame::new(0);
        frame.phase = TddPhase::Done;
        let result = tdd_step_action(&frame, "login slice", &routing).expect("no error");
        assert!(result.is_none());
    }

    #[test]
    fn tdd_write_test_missing_route_returns_routing_error() {
        let routing = routing_with(&[]); // empty — no OuterBehavioralTestWriting
        let frame = TddFrame::new(0); // starts at WriteTest
        let err = tdd_step_action(&frame, "login slice", &routing)
            .expect_err("should fail");
        assert!(matches!(err, StepError::Routing(_)));
    }

    #[test]
    fn tdd_write_test_prompt_contains_description() {
        let routing = routing_with(&[(WorkType::OuterBehavioralTestWriting, "writer")]);
        let frame = TddFrame::new(0);
        if let StepAction::SpawnAgent { prompt, .. } =
            tdd_step_action(&frame, "unique-slice-42", &routing)
                .expect("step")
                .expect("Some")
        {
            assert!(prompt.to_string().contains("unique-slice-42"));
        } else {
            panic!("expected SpawnAgent");
        }
    }
}
