//! Factory event types and file-system persistence.
//!
//! Events are the source of truth. Each event is appended as a JSON file in
//! `.claude-factory/events/v1/`. The in-memory projection is rebuilt by
//! replaying these files on startup.
//!
//! File naming: `{sequence:010}-{uuid}.json` so lexicographic order equals
//! chronological order.

use crate::store::event_export_dir;
use cfk_core::{
    state_machine::work_item::WorkItem,
    types::{
        ids::{LeaseId, ProjectId, WorkItemId},
        lease::Lease,
    },
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

/// An event in the factory's audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FactoryEvent {
    /// A project was initialized in a product repo.
    ProjectInitialized { id: ProjectId },
    /// A work item was added to the backlog.
    WorkItemAdded { work_item: WorkItem },
    /// A lease was granted; the work item is now `InProgress`.
    LeaseGranted { lease: Lease },
    /// A lease was released; the work item reverts to `Ready`.
    LeaseReleased {
        lease_id: LeaseId,
        work_item_id: WorkItemId,
    },
    /// A work item was completed.
    WorkItemCompleted { work_item_id: WorkItemId },
    /// A work item was abandoned (superseded or cancelled).
    WorkItemAbandoned { work_item_id: WorkItemId },
}

/// A persisted event with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// Stable unique identifier for this event (also used in the filename).
    pub id: Uuid,
    /// Monotonically-increasing sequence number within this project.
    pub sequence: u64,
    /// Wall-clock time at which the event was written.
    pub timestamp: DateTime<Utc>,
    /// The event itself.
    pub payload: FactoryEvent,
}

impl EventEnvelope {
    fn filename(&self) -> String {
        format!("{:010}-{}.json", self.sequence, self.id)
    }
}

/// Load all persisted events from `dir`, sorted by sequence number.
///
/// Returns an empty `Vec` if the directory does not exist.
///
/// # Errors
/// Returns an error if any event file cannot be read or parsed.
pub fn load_events(dir: &Path) -> anyhow::Result<Vec<EventEnvelope>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();

    entries.sort_by_key(std::fs::DirEntry::file_name);

    let mut events = Vec::with_capacity(entries.len());
    for entry in entries {
        let path = entry.path();
        let content = std::fs::read_to_string(&path)?;
        let envelope: EventEnvelope = serde_json::from_str(&content).map_err(|e| {
            anyhow::anyhow!("failed to parse {}: {}", path.display(), e)
        })?;
        events.push(envelope);
    }

    Ok(events)
}

/// Append one event to `.claude-factory/events/v1/` and return its envelope.
///
/// # Errors
/// Returns an error if the event directory cannot be created or the file cannot
/// be written.
pub fn append_event(
    project_root: &Path,
    sequence: u64,
    payload: FactoryEvent,
) -> anyhow::Result<EventEnvelope> {
    let dir = event_export_dir(project_root);
    std::fs::create_dir_all(&dir)?;

    let envelope = EventEnvelope {
        id: Uuid::new_v4(),
        sequence,
        timestamp: Utc::now(),
        payload,
    };

    let path = dir.join(envelope.filename());
    let content = serde_json::to_string_pretty(&envelope)?;
    std::fs::write(path, content)?;

    Ok(envelope)
}
