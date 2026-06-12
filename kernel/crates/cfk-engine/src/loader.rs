//! Project state replay from the event log.
//!
//! `load_project_state` reads all event files from `.claude-factory/events/v1/`,
//! applies them in order to a fresh `ProjectState`, and returns the resulting
//! projection. Returns `None` if the project has not been initialized.

use crate::{
    config::default_routing_table,
    events::{FactoryEvent, load_events},
    project::ProjectState,
    store::event_export_dir,
};
use cfk_core::{
    state_machine::{
        architecture::AdrPhase,
        design::DesignPhase,
        discovery::DiscoveryPhase,
        review::{ReviewSlicePhase, ReviewSliceState},
        work_item::WorkItemStatus,
    },
    types::{
        architecture::{AdrRecord, AdrStatus},
        design::{ComponentName, DesignComponent},
        gate::GateKind,
        tdd::{DevSliceState, TddFrame, TddPhase},
    },
};
use std::path::Path;

/// Replay all events and return the current `ProjectState`.
///
/// Returns `None` if `.claude-factory/events/v1/` does not exist or contains
/// no `ProjectInitialized` event.
///
/// # Errors
/// Returns an error if any event file cannot be read or deserialized.
pub fn load_project_state(root: &Path) -> anyhow::Result<Option<ProjectState>> {
    let dir = event_export_dir(root);
    let envelopes = load_events(&dir)?;

    let Some(first) = envelopes.first() else {
        return Ok(None);
    };

    let FactoryEvent::ProjectInitialized { id } = &first.payload else {
        return Ok(None);
    };

    let routing = default_routing_table();
    let mut state = ProjectState::new(id.clone(), root.to_path_buf(), routing);

    for envelope in envelopes.iter().skip(1) {
        apply_event(&mut state, &envelope.payload);
    }

    Ok(Some(state))
}

/// Apply one event to the in-memory projection in place.
#[expect(clippy::too_many_lines, reason = "exhaustive match over all FactoryEvent variants; each arm is a simple projection step and cannot be meaningfully split")]
pub fn apply_event(state: &mut ProjectState, event: &FactoryEvent) {
    match event {
        FactoryEvent::ProjectInitialized { .. }
        | FactoryEvent::DesignCrossCheckCompleted { .. } => {}

        FactoryEvent::StepOutcomeRecorded { work_type, outcome, tokens_used } => {
            state
                .metrics
                .entry(*work_type)
                .or_default()
                .record(*outcome, *tokens_used);
        }

        FactoryEvent::WorkItemAdded { work_item } => {
            state.work_items.push(work_item.clone());
        }

        FactoryEvent::LeaseGranted { lease } => {
            let wid = &lease.work_item_id;
            if let Some(item) = state.work_items.iter_mut().find(|i| &i.id == wid) {
                item.status = WorkItemStatus::InProgress;
                item.active_lease = Some(lease.id.clone());
            }
            state.leases.push(lease.clone());
        }

        FactoryEvent::LeaseReleased { lease_id, work_item_id } => {
            state.leases.retain(|l| &l.id != lease_id);
            if let Some(item) = state.work_items.iter_mut().find(|i| &i.id == work_item_id) {
                item.status = WorkItemStatus::Ready;
                item.active_lease = None;
            }
        }

        FactoryEvent::WorkItemCompleted { work_item_id } => {
            if let Some(item) = state.work_items.iter_mut().find(|i| &i.id == work_item_id) {
                item.status = WorkItemStatus::Done;
                item.active_lease = None;
                item.active_step = None;
            }
            state.dev_states.remove(work_item_id);
        }

        FactoryEvent::WorkItemAbandoned { work_item_id } => {
            if let Some(item) = state.work_items.iter_mut().find(|i| &i.id == work_item_id) {
                item.status = WorkItemStatus::Abandoned;
                item.active_lease = None;
                item.active_step = None;
            }
            state.dev_states.remove(work_item_id);
        }

        // ── TDD events ───────────────────────────────────────────────────

        FactoryEvent::TddSliceStarted { work_item_id, author_identity } => {
            let mut dev = DevSliceState::new(work_item_id.clone());
            if let Some(frame) = dev.current_frame_mut() {
                frame.author_identity = Some(author_identity.clone());
            }
            state.dev_states.insert(work_item_id.clone(), dev);
        }

        FactoryEvent::TddPhaseAdvanced { work_item_id, frame_depth, new_phase } => {
            if let Some(dev) = state.dev_states.get_mut(work_item_id)
                && let Some(frame) = dev.frames.iter_mut().find(|f| f.depth == *frame_depth)
            {
                frame.phase = new_phase.clone();
            }
        }

        FactoryEvent::TddTestSubmitted { work_item_id, frame_depth, test_content, author_identity } => {
            if let Some(dev) = state.dev_states.get_mut(work_item_id)
                && let Some(frame) = dev.frames.iter_mut().find(|f| f.depth == *frame_depth)
            {
                frame.test_content = Some(test_content.clone());
                frame.author_identity = Some(author_identity.clone());
                frame.phase = TddPhase::TestReviewGate;
            }
        }

        FactoryEvent::TddGateVerdict { work_item_id, gate_kind, verdict, reviewer_id: _ } => {
            if let Some(dev) = state.dev_states.get_mut(work_item_id)
                && let Some(frame) = dev.current_frame_mut()
            {
                frame.phase = match gate_kind {
                    GateKind::TestReview => {
                        if verdict.is_approved() { TddPhase::RedCheck } else { TddPhase::WriteTest }
                    }
                    GateKind::ImplementationReview => {
                        if verdict.is_approved() { TddPhase::LintCheck } else { TddPhase::Implement }
                    }
                    // AdrReview verdicts are not TDD events; no TDD phase change.
                    GateKind::AdrReview => frame.phase.clone(),
                };
            }
        }

        FactoryEvent::TddCheckResult { work_item_id, check_name: _, passed, first_error } => {
            if let Some(dev) = state.dev_states.get_mut(work_item_id)
                && let Some(frame) = dev.current_frame_mut()
            {
                let typed_error = first_error.clone();
                match (&frame.phase.clone(), passed) {
                    (TddPhase::RedCheck, false) => {
                        frame.expected_failure.clone_from(&typed_error);
                        frame.current_error = typed_error;
                        frame.phase = TddPhase::Implement;
                    }
                    (TddPhase::CheckProgress | TddPhase::Implement, true) => {
                        frame.phase = TddPhase::ImplReviewGate;
                    }
                    (TddPhase::CheckProgress | TddPhase::Implement | TddPhase::LintCheck, false) => {
                        frame.current_error = typed_error;
                        frame.phase = TddPhase::Implement;
                    }
                    (TddPhase::LintCheck, true) => {
                        frame.phase = TddPhase::Done;
                    }
                    _ => {}
                }
            }
        }

        FactoryEvent::TddDrillDownPushed { work_item_id, child_depth, child_description: _ } => {
            if let Some(dev) = state.dev_states.get_mut(work_item_id) {
                dev.frames.push(TddFrame::new(*child_depth));
            }
        }

        FactoryEvent::TddDrillDownPopped { work_item_id } => {
            if let Some(dev) = state.dev_states.get_mut(work_item_id) {
                dev.frames.pop();
                if let Some(parent) = dev.frames.last_mut() {
                    parent.phase = TddPhase::Implement;
                }
            }
        }

        FactoryEvent::TddSliceDone { work_item_id } => {
            if let Some(dev) = state.dev_states.get_mut(work_item_id)
                && let Some(frame) = dev.current_frame_mut()
            {
                frame.phase = TddPhase::Done;
            }
            // Work item completion is handled by a separate WorkItemCompleted event.
        }

        // ── Review events ────────────────────────────────────────────────

        FactoryEvent::ReviewSliceStarted { work_item_id, pr_number, pr_url } => {
            let mut review = ReviewSliceState::new(work_item_id.clone());
            review.phase = ReviewSlicePhase::PrOpen;
            review.pr_number = Some(pr_number.into_inner());
            review.pr_url = Some(pr_url.to_string());
            state.review_states.insert(work_item_id.clone(), review);
        }

        FactoryEvent::ReviewCommentTriageCreated {
            review_work_item_id,
            triage_item_id,
            comment_id,
            comment_body: _,
        } => {
            if let Some(review) = state.review_states.get_mut(review_work_item_id) {
                review.seen_comment_ids.push(comment_id.to_string());
                review.pending_triage.push((comment_id.to_string(), triage_item_id.clone()));
                review.phase = ReviewSlicePhase::CommentTriagePending;
            }
        }

        FactoryEvent::ReviewCommentPosted {
            review_work_item_id,
            comment_id,
            triage_item_id,
        } => {
            if let Some(review) = state.review_states.get_mut(review_work_item_id) {
                let cid_str = comment_id.to_string();
                review.pending_triage.retain(|(cid, tid)| {
                    *cid != cid_str || tid != triage_item_id
                });
                if review.pending_triage.is_empty() {
                    review.phase = ReviewSlicePhase::PrOpen;
                }
            }
        }

        FactoryEvent::ReviewAllGreen { work_item_id } => {
            if let Some(review) = state.review_states.get_mut(work_item_id) {
                review.phase = ReviewSlicePhase::AllGreen;
            }
        }

        FactoryEvent::ReviewPrMerged { work_item_id } => {
            if let Some(review) = state.review_states.get_mut(work_item_id) {
                review.phase = ReviewSlicePhase::Merged;
            }
            // Work item completion is handled by a separate WorkItemCompleted event.
        }

        // ── Discovery events ─────────────────────────────────────────────

        FactoryEvent::DiscoveryBriefDrafted { work_item_id, brief_content, workflows } => {
            let disc = state
                .discovery_states
                .entry(work_item_id.clone())
                .or_default();
            disc.phase = DiscoveryPhase::BriefReady;
            disc.brief_content = Some(brief_content.clone());
            disc.workflows.clone_from(workflows);
        }

        FactoryEvent::DiscoveryApproved { work_item_id } => {
            if let Some(disc) = state.discovery_states.get_mut(work_item_id) {
                disc.phase = DiscoveryPhase::Approved;
            }
            // Workflow work items are added separately as WorkItemAdded events.
        }

        // ── Architecture events ──────────────────────────────────────────

        FactoryEvent::AdrDrafted { work_item_id, adr_id, title, content } => {
            let adr_state = state
                .adr_states
                .entry(work_item_id.clone())
                .or_default();
            adr_state.phase = AdrPhase::PendingReview;
            adr_state.adr_id = Some(adr_id.clone());
            adr_state.title = Some(title.clone());
            adr_state.content = Some(content.clone());
            // Add to global ADR registry as proposed.
            state.adrs.push(AdrRecord {
                id: adr_id.clone(),
                work_item_id: work_item_id.clone(),
                title: title.clone(),
                content: content.clone(),
                status: AdrStatus::Proposed,
            });
        }

        FactoryEvent::AdrDecided { work_item_id, adr_id, accepted, reason: _ } => {
            if let Some(adr_state) = state.adr_states.get_mut(work_item_id) {
                adr_state.phase = if *accepted { AdrPhase::Accepted } else { AdrPhase::Rejected };
            }
            // Update status in global ADR registry.
            if let Some(rec) = state.adrs.iter_mut().find(|r| &r.id == adr_id) {
                rec.status = if *accepted { AdrStatus::Accepted } else { AdrStatus::Rejected };
            }
        }

        // ── Design-system events ─────────────────────────────────────────

        FactoryEvent::DesignComponentAdded {
            work_item_id,
            component_id,
            name,
            kind,
            slice_ref,
        } => {
            let ds = state
                .design_states
                .entry(work_item_id.clone())
                .or_default();
            ds.phase = DesignPhase::Done;
            ds.component_name = ComponentName::try_new(name.clone()).ok();
            if let Ok(component_name) = ComponentName::try_new(name.clone()) {
                state.design_inventory.push(DesignComponent {
                    id: component_id.clone(),
                    name: component_name,
                    kind: *kind,
                    slice_ref: slice_ref.clone(),
                });
            }
        }

    }
}
