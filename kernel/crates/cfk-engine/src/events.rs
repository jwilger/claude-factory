//! Factory event types and file-system persistence.
//!
//! Events are the source of truth. Each event is appended as a JSON file in
//! `.claude-factory/events/v1/`. The in-memory projection is rebuilt by
//! replaying these files on startup.
//!
//! File naming: `{sequence:010}-{uuid}.json` so lexicographic order equals
//! chronological order.

use crate::store::event_export_dir;
use eventcore_fs::FileEventStore;
use eventcore_types::{Event, EventStore, StreamId, StreamVersion, StreamWrites};
use std::io;
use std::path::PathBuf;
use thiserror::Error;
use cfk_core::{
    state_machine::work_item::WorkItem,
    types::{
        forge::{CommentBody, CommentId, PrNumber, PrUrl},
        gate::{GateKind, GateVerdict},
        ids::{LeaseId, ProjectId, WorkItemId},
        lease::Lease,
        metrics::StepOutcome,
        routing::WorkType,
        step::CheckName,
        tdd::{AuthorIdentity, DrillDownDescription, ErrorMessage, ReviewerId, TestCode, TddPhase},
    },
};
/// Error returned by event-store operations.
#[derive(Debug, Error)]
pub enum EventStoreError {
    #[error("failed to read event directory {dir}: {source}")]
    ReadDir { dir: PathBuf, #[source] source: io::Error },
    #[error("failed to read event file {path}: {source}")]
    ReadFile { path: PathBuf, #[source] source: io::Error },
    #[error("failed to parse event file {path}: {source}")]
    ParseEvent { path: PathBuf, #[source] source: serde_json::Error },
    #[error("failed to serialize event: {0}")]
    SerializeEvent(serde_json::Error),
    #[error("failed to create event directory {dir}: {source}")]
    CreateDir { dir: PathBuf, #[source] source: io::Error },
    #[error("failed to write event file {path}: {source}")]
    WriteFile { path: PathBuf, #[source] source: io::Error },
    #[error("v2 event store error: {0}")]
    V2Append(String),
}

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
        author_identity: AuthorIdentity,
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
        test_content: TestCode,
        author_identity: AuthorIdentity,
    },
    /// A gate reviewer recorded a verdict.
    TddGateVerdict {
        work_item_id: WorkItemId,
        gate_kind: GateKind,
        verdict: GateVerdict,
        reviewer_id: ReviewerId,
    },
    /// The kernel ran a check and recorded the result.
    TddCheckResult {
        work_item_id: WorkItemId,
        check_name: CheckName,
        passed: bool,
        first_error: Option<ErrorMessage>,
    },
    /// A drill-down frame was pushed (implementer needs tighter unit test).
    TddDrillDownPushed {
        work_item_id: WorkItemId,
        child_description: DrillDownDescription,
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
        pr_number: PrNumber,
        pr_url: PrUrl,
    },
    /// A new PR comment was triaged — a `PrCommentTriage` work item was created.
    ReviewCommentTriageCreated {
        review_work_item_id: WorkItemId,
        triage_item_id: WorkItemId,
        comment_id: CommentId,
        comment_body: CommentBody,
    },
    /// The kernel posted a reply to a PR comment (triage item completed).
    ReviewCommentPosted {
        review_work_item_id: WorkItemId,
        comment_id: CommentId,
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

    // ── Metrics events ───────────────────────────────────────────────────
    /// A step outcome was recorded for routing-table metrics.
    ///
    /// The conductor calls `cf_record_outcome` after each step completes to
    /// accumulate veto rates and token costs per work type. This data justifies
    /// routing defaults and guides tuning decisions.
    StepOutcomeRecorded {
        work_type: WorkType,
        outcome: StepOutcome,
        /// Tokens consumed by the agent for this step, if reported.
        tokens_used: Option<u32>,
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
pub fn load_events(dir: &Path) -> Result<Vec<EventEnvelope>, EventStoreError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|source| EventStoreError::ReadDir { dir: dir.to_path_buf(), source })?
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();

    entries.sort_by_key(std::fs::DirEntry::file_name);

    let mut events = Vec::with_capacity(entries.len());
    for entry in entries {
        let path = entry.path();
        let content = std::fs::read_to_string(&path)
            .map_err(|source| EventStoreError::ReadFile { path: path.clone(), source })?;
        let envelope: EventEnvelope = serde_json::from_str(&content)
            .map_err(|source| EventStoreError::ParseEvent { path, source })?;
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
) -> Result<EventEnvelope, EventStoreError> {
    let dir = event_export_dir(project_root);
    std::fs::create_dir_all(&dir)
        .map_err(|source| EventStoreError::CreateDir { dir: dir.clone(), source })?;

    let envelope = EventEnvelope {
        id: Uuid::new_v4(),
        sequence,
        timestamp: Utc::now(),
        payload,
    };

    let path = dir.join(envelope.filename());
    let content = serde_json::to_string_pretty(&envelope)
        .map_err(EventStoreError::SerializeEvent)?;
    std::fs::write(&path, content)
        .map_err(|source| EventStoreError::WriteFile { path, source })?;

    Ok(envelope)
}

// ── eventcore-types Event impl ────────────────────────────────────────────────

static FACTORY_STREAM_ID: std::sync::OnceLock<StreamId> = std::sync::OnceLock::new();

impl Event for FactoryEvent {
    fn stream_id(&self) -> &StreamId {
        FACTORY_STREAM_ID.get_or_init(|| {
            #[expect(
                clippy::expect_used,
                reason = "static stream id literal 'factory-events' is always valid; failure is impossible at runtime"
            )]
            StreamId::try_new("factory-events".to_string())
                .expect("valid stream id literal")
        })
    }

    fn event_type_name() -> &'static str {
        "FactoryEvent"
    }
}

// ── v2 eventcore-fs store helpers ─────────────────────────────────────────────

/// Append one event to the v2 `eventcore-fs` store.
///
/// `expected_version` is the number of events already in the stream
/// (0 = empty, 1 = one event exists, etc.).
///
/// # Errors
/// Returns an error if the append fails.
pub async fn append_event_v2(
    store: &FileEventStore,
    event: FactoryEvent,
    expected_version: usize,
) -> Result<(), EventStoreError> {
    let stream_id = event.stream_id().clone();
    let writes = StreamWrites::new()
        .register_stream(stream_id, StreamVersion::new(expected_version))
        .map_err(|e| EventStoreError::V2Append(e.to_string()))?
        .append(event)
        .map_err(|e| EventStoreError::V2Append(e.to_string()))?;
    store
        .append_events(writes)
        .await
        .map_err(|e| EventStoreError::V2Append(e.to_string()))?;
    Ok(())
}

/// Migrate all v1 events to the v2 `eventcore-fs` store.
///
/// Reads events from `.claude-factory/events/v1/`, appends them one-by-one to
/// the v2 store. Safe to call repeatedly — if the v2 store already has events
/// (stream not at version 0) the migration is skipped.
///
/// # Errors
/// Returns an error if v1 events cannot be read or v2 append fails.
pub async fn migrate_v1_to_v2(
    project_root: &Path,
    store: &FileEventStore,
) -> Result<(), EventStoreError> {
    let v1_events = load_events(&event_export_dir(project_root))?;

    let Some(first_envelope) = v1_events.first() else {
        return Ok(());
    };

    let first_event = first_envelope.payload.clone();
    let stream_id = first_event.stream_id().clone();

    let probe = StreamWrites::new()
        .register_stream(stream_id.clone(), StreamVersion::new(0))
        .map_err(|e| EventStoreError::V2Append(e.to_string()))?
        .append(first_event)
        .map_err(|e| EventStoreError::V2Append(e.to_string()))?;

    if store.append_events(probe).await.is_err() {
        // Version conflict — migration already ran; nothing to do.
        return Ok(());
    }

    // Append remaining events one at a time, each as a single-event transaction.
    for (i, envelope) in v1_events.iter().enumerate().skip(1) {
        let event = envelope.payload.clone();
        let writes = StreamWrites::new()
            .register_stream(stream_id.clone(), StreamVersion::new(i))
            .map_err(|e| EventStoreError::V2Append(e.to_string()))?
            .append(event)
            .map_err(|e| EventStoreError::V2Append(e.to_string()))?;
        store
            .append_events(writes)
            .await
            .map_err(|e| EventStoreError::V2Append(e.to_string()))?;
    }

    Ok(())
}
