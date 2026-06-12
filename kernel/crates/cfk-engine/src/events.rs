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
        gate::{GateKind, GateVerdict},
        ids::{LeaseId, ProjectId, WorkItemId},
        lease::Lease,
        tdd::TddPhase,
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

    // ── TDD slice events ─────────────────────────────────────────────────
    /// A development slice was claimed and its TDD cycle started.
    TddSliceStarted {
        work_item_id: WorkItemId,
        author_identity: String,
    },
    /// The TDD frame advanced to a new phase.
    TddPhaseAdvanced {
        work_item_id: WorkItemId,
        frame_depth: u32,
        new_phase: TddPhase,
    },
    /// Test-writer agent submitted test code.
    TddTestSubmitted {
        work_item_id: WorkItemId,
        frame_depth: u32,
        test_content: String,
        author_identity: String,
    },
    /// A gate reviewer recorded a verdict.
    TddGateVerdict {
        work_item_id: WorkItemId,
        gate_kind: GateKind,
        verdict: GateVerdict,
        reviewer_id: String,
    },
    /// The kernel ran a check and recorded the result.
    TddCheckResult {
        work_item_id: WorkItemId,
        check_name: String,
        passed: bool,
        first_error: Option<String>,
    },
    /// A drill-down frame was pushed (implementer needs tighter unit test).
    TddDrillDownPushed {
        work_item_id: WorkItemId,
        child_description: String,
        child_depth: u32,
    },
    /// The innermost drill-down frame completed; parent resumes.
    TddDrillDownPopped { work_item_id: WorkItemId },
    /// All TDD frames complete; slice is done (work item proceeds to commit).
    TddSliceDone { work_item_id: WorkItemId },

    // ── Review phase events ──────────────────────────────────────────────
    /// A review slice was started and a PR was opened.
    ReviewSliceStarted {
        work_item_id: WorkItemId,
        pr_number: u64,
        pr_url: String,
    },
    /// A new PR comment was triaged — a `PrCommentTriage` work item was created.
    ReviewCommentTriageCreated {
        review_work_item_id: WorkItemId,
        triage_item_id: WorkItemId,
        comment_id: String,
        comment_body: String,
    },
    /// The kernel posted a reply to a PR comment (triage item completed).
    ReviewCommentPosted {
        review_work_item_id: WorkItemId,
        comment_id: String,
        triage_item_id: WorkItemId,
    },
    /// All CI checks passed and the PR is approved.
    ReviewAllGreen { work_item_id: WorkItemId },
    /// The PR was merged; the slice is done.
    ReviewPrMerged { work_item_id: WorkItemId },

    // ── Discovery phase events ───────────────────────────────────────────
    /// The discovery agent submitted a product brief with workflow list.
    DiscoveryBriefDrafted {
        work_item_id: WorkItemId,
        brief_content: String,
        /// Workflow names to be queued for event modeling on approval.
        workflows: Vec<String>,
    },
    /// Human approved the discovery brief; workflows are queued.
    DiscoveryApproved { work_item_id: WorkItemId },

    // ── Architecture phase events ────────────────────────────────────────
    /// An architect agent submitted an ADR draft.
    AdrDrafted {
        work_item_id: WorkItemId,
        adr_id: cfk_core::types::ids::AdrId,
        title: String,
        content: String,
    },
    /// A reviewer gate recorded a verdict on an ADR.
    AdrDecided {
        work_item_id: WorkItemId,
        adr_id: cfk_core::types::ids::AdrId,
        accepted: bool,
        reason: Option<String>,
    },

    // ── Design-system phase events ───────────────────────────────────────
    /// A design component was added to the inventory.
    DesignComponentAdded {
        work_item_id: WorkItemId,
        component_id: cfk_core::types::ids::ComponentId,
        name: String,
        kind: cfk_core::types::design::AtomicKind,
        slice_ref: Option<String>,
    },
    /// The design cross-check ran and generated work items for gaps.
    DesignCrossCheckCompleted {
        /// IDs of work items generated for missing components.
        generated_item_ids: Vec<WorkItemId>,
    },
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
