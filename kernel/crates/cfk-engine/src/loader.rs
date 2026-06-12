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
    state_machine::work_item::WorkItemStatus,
    types::{
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
#[allow(clippy::too_many_lines)]
pub fn apply_event(state: &mut ProjectState, event: &FactoryEvent) {
    match event {
        FactoryEvent::ProjectInitialized { .. } => {}

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
                test_content.clone_into(frame.test_content.insert(String::new()));
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
                };
            }
        }

        FactoryEvent::TddCheckResult { work_item_id, check_name: _, passed, first_error } => {
            if let Some(dev) = state.dev_states.get_mut(work_item_id)
                && let Some(frame) = dev.current_frame_mut()
            {
                match (&frame.phase.clone(), passed) {
                    (TddPhase::RedCheck, false) => {
                        frame.expected_failure.clone_from(first_error);
                        frame.current_error.clone_from(first_error);
                        frame.phase = TddPhase::Implement;
                    }
                    (TddPhase::CheckProgress | TddPhase::Implement, true) => {
                        frame.phase = TddPhase::ImplReviewGate;
                    }
                    (TddPhase::CheckProgress | TddPhase::Implement | TddPhase::LintCheck, false) => {
                        frame.current_error.clone_from(first_error);
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
    }
}
