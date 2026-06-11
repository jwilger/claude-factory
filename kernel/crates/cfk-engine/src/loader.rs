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
use cfk_core::state_machine::work_item::WorkItemStatus;
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
        FactoryEvent::LeaseReleased {
            lease_id,
            work_item_id,
        } => {
            state.leases.retain(|l| &l.id != lease_id);
            if let Some(item) = state
                .work_items
                .iter_mut()
                .find(|i| &i.id == work_item_id)
            {
                item.status = WorkItemStatus::Ready;
                item.active_lease = None;
            }
        }
        FactoryEvent::WorkItemCompleted { work_item_id } => {
            if let Some(item) = state
                .work_items
                .iter_mut()
                .find(|i| &i.id == work_item_id)
            {
                item.status = WorkItemStatus::Done;
                item.active_lease = None;
                item.active_step = None;
            }
        }
        FactoryEvent::WorkItemAbandoned { work_item_id } => {
            if let Some(item) = state
                .work_items
                .iter_mut()
                .find(|i| &i.id == work_item_id)
            {
                item.status = WorkItemStatus::Abandoned;
                item.active_lease = None;
                item.active_step = None;
            }
        }
    }
}
