//! MCP server implementation — exposes `cf_*` tools over stdio.

use cfk_core::{
    promotion::next_slice_promotion,
    state_machine::{
        architecture::AdrPhase,
        discovery::DiscoveryPhase,
        work_item::{WorkItem, WorkItemStatus},
    },
    types::{
        design::{AtomicKind, ComponentOwnership},
        gate::{GateKind, GateVerdict, VetoReason},
        ids::{AdrId, ComponentId, WorkItemId},
        lease::SessionIdentity,
        metrics::StepOutcome,
        phase::PhaseKind,
        routing::WorkType,
        forge::{CommentBody, PrNumber, PrUrl},
        step::CheckName,
        tdd::{AuthorIdentity, DrillDownDescription, ErrorMessage, ReviewerId, TestCode, TddPhase},
    },
};
use cfk_engine::{
    architecture::project_architecture_md,
    checks::load_checks,
    commands::{CommandError, SubmissionOutcome, SubmissionPayload, handle_claim, handle_metrics, handle_next_step, handle_record_outcome, handle_submission, normalize_submission_result, validate_gate_verdict},
    config::default_routing_table,
    emc::{read_verified_slices, EmcSlice},
    events::{FactoryEvent, append_event_v2},
    forge::{ForgeAdapter, GiteaForge},
    loader::{apply_event, load_project_state_v2},
    project::ProjectState,
    review::{handle_pr_merge, handle_pr_poll},
    runner::run_check,
    store::eventcore_store_dir,
};
use eventcore_fs::FileEventStore;
use rmcp::{
    ErrorData as McpError,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars,
    tool, tool_handler, tool_router,
    ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

// ── Tool parameter types ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct InitParams {
    /// Absolute path to the product repo root. Uses the kernel's working
    /// directory when omitted.
    pub project_root: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PhaseFilterParams {
    /// Restrict results to a specific phase.
    pub phase: Option<PhaseKind>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NextStepParams {
    /// Optional phase filter.
    pub phase: Option<PhaseKind>,
    /// Session identity for this conductor session (used for auto-claiming
    /// development work items). Required when development items are present.
    pub session_identity: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ClaimParams {
    /// The `work_item_id` from a previous `cf_next_step` call.
    pub work_item_id: String,
    /// Human-readable session identifier (e.g. hostname + pid).
    pub session_identity: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WorkItemIdParams {
    /// The `work_item_id` of the target work item.
    pub work_item_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AbandonParams {
    /// The `work_item_id` of the work item to abandon.
    pub work_item_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SubmitParams {
    /// The `work_item_id` of the item being submitted.
    pub work_item_id: String,
    /// Phase-specific result data (JSON).
    ///
    /// For `WriteTest`: `{"test_content": "..."}`.
    /// For `Implement`: `{"drill_down_description": null | "..."}`.
    /// For other phases: any value (recorded as evidence).
    pub result: serde_json::Value,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GateParams {
    /// The `work_item_id` under review.
    pub work_item_id: String,
    /// Session identity of the reviewer (must differ from the work item author).
    pub reviewer_id: String,
    /// `"approved"` or `"vetoed"`.
    pub verdict: String,
    /// Required when `verdict` is `"vetoed"`. Explains why.
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BacklogAddParams {
    /// Phase for this work item.
    pub phase: PhaseKind,
    /// Work type.
    pub work_type: WorkType,
    /// Human-readable description of the work.
    pub description: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IngestSlicesParams {
    /// Absolute path to the product repo root containing `model/events/v1/`.
    /// Defaults to the kernel's configured project root.
    pub project_root: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RunCheckParams {
    /// Name of the configured check to run (e.g. `tests`, `lint`).
    pub check_name: String,
    /// Work item that this check result belongs to (if any).
    /// When provided, the kernel advances the TDD state machine.
    pub work_item_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PrOpenParams {
    /// The `work_item_id` of the review slice.
    pub work_item_id: String,
    /// Pull request title.
    pub title: String,
    /// Pull request body (description).
    pub body: String,
    /// Head branch name (the branch with the changes).
    pub head: String,
    /// Base branch to merge into (e.g. `main`).
    pub base: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PrPollParams {
    /// The `work_item_id` of the review slice.
    pub work_item_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PrMergeParams {
    /// The `work_item_id` of the review slice.
    pub work_item_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DiscoverySubmitParams {
    /// The `work_item_id` of the discovery work item.
    pub work_item_id: String,
    /// The product brief (context, risks, opportunity summary).
    pub brief_content: String,
    /// Workflow names to queue for event modeling (e.g. `["place order", "track shipment"]`).
    pub workflows: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DiscoveryApproveParams {
    /// The `work_item_id` of the discovery work item.
    pub work_item_id: String,
    /// `true` to approve and queue workflows; `false` to reset for re-run.
    pub approved: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AdrSubmitParams {
    /// The `work_item_id` of the architecture work item.
    pub work_item_id: String,
    /// Short title for the ADR (e.g. `"Use PostgreSQL for event store"`).
    pub title: String,
    /// Full ADR content (Context / Decision / Consequences format).
    pub content: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TriageSubmitParams {
    /// The `work_item_id` of the triage work item (ArchitectureTriage/DesignTriage).
    pub work_item_id: String,
    /// `true` if the slice needs follow-up work — an ADR for architecture triage,
    /// or design components for design triage. `false` fast-passes the gate.
    pub needs_followup: bool,
    /// One-paragraph rationale for the decision (retained in the event log).
    pub rationale: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DesignAddComponentParams {
    /// The `work_item_id` of the design-system work item.
    pub work_item_id: String,
    /// Component name (e.g. `"PrimaryButton"`).
    pub name: String,
    /// Atomic Design level: `quark` | `atom` | `molecule` | `organism` | `template` | `page`.
    pub kind: String,
    /// Owning layer (ADR 0012): `platform` (reusable UI library) or `slice`
    /// (bespoke to the slice). Defaults to `slice` when omitted.
    pub ownership: Option<String>,
    /// Optional emc slice slug this component satisfies.
    pub slice_ref: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DesignCrossCheckParams {
    /// Workflow names whose slices should be cross-checked for missing components.
    pub workflows: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RecordOutcomeParams {
    /// Work item ID the step belongs to.
    pub work_item_id: String,
    /// Step outcome: "approved", "vetoed", or "completed".
    pub outcome: String,
    /// Tokens consumed by the agent for this step, if known.
    pub tokens_used: Option<u32>,
}

// ── Response types ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct PhaseStatusEntry {
    phase: PhaseKind,
    ready: usize,
    in_progress: usize,
    done: usize,
    abandoned: usize,
}

#[derive(Debug, Serialize)]
struct StatusResponse {
    project_id: String,
    phases: Vec<PhaseStatusEntry>,
}

#[derive(Debug, Serialize)]
struct WorkItemSummary {
    id: String,
    phase: String,
    work_type: String,
    status: String,
    description: String,
}

// ── Server state ──────────────────────────────────────────────────────────

struct ServerState {
    project_root: PathBuf,
    project: Option<ProjectState>,
    event_store: FileEventStore,
    v2_stream_version: usize,
}

impl ServerState {
    fn new(
        project_root: PathBuf,
        project: Option<ProjectState>,
        event_store: FileEventStore,
        v2_stream_version: usize,
    ) -> Self {
        Self { project_root, project, event_store, v2_stream_version }
    }

    async fn emit(&mut self, event: FactoryEvent) -> anyhow::Result<()> {
        append_event_v2(&self.event_store, event.clone(), self.v2_stream_version).await?;
        self.v2_stream_version += 1;
        if let Some(ref mut proj) = self.project {
            apply_event(proj, &event);
        }
        Ok(())
    }
}

// ── Server ────────────────────────────────────────────────────────────────

/// The Claude-Factory kernel MCP server.
#[derive(Clone)]
pub struct CfkServer {
    state: Arc<RwLock<ServerState>>,
    forge: Arc<dyn ForgeAdapter>,
    /// The Claude Code session id this server runs under, if any. Leases are
    /// keyed to it so they match the session the `PreToolUse` guardrail checks.
    /// `None` outside Claude Code (manual runs, tests), where the caller-provided
    /// identity is used instead.
    session_identity: Option<String>,
    // Used by the `#[tool_handler]` macro; dead-code lint doesn't see the use.
    #[expect(dead_code, reason = "field is read by the #[tool_handler] macro expansion; rustc's dead-code analysis does not see macro-generated references")]
    tool_router: ToolRouter<Self>,
}

impl CfkServer {
    /// Load existing project state (if any) and return a ready server.
    ///
    /// Uses `GiteaForge` when `GITEA_URL`/`GITEA_TOKEN`/`GITEA_OWNER`/`GITEA_REPO`
    /// are set. Fails fast if none are configured — `MemoryForge` is test-only
    /// and must never run in production (it silently swallows PR operations).
    ///
    /// # Errors
    /// Returns an error if no forge is configured or the event log cannot be read.
    pub async fn load(project_root: PathBuf) -> anyhow::Result<Self> {
        let forge: Arc<dyn ForgeAdapter> = GiteaForge::from_env()
            .map_err(|e| anyhow::anyhow!(
                "no forge configured ({e}); set GITEA_URL, GITEA_TOKEN, GITEA_OWNER, \
                 and GITEA_REPO to connect to a Forgejo/Gitea instance"
            ))?;
        // Bind leases to the Claude Code session id (what the guardrail checks).
        let session_identity = std::env::var("CLAUDE_CODE_SESSION_ID")
            .ok()
            .filter(|s| !s.trim().is_empty());
        Self::load_with_forge_and_session(project_root, forge, session_identity).await
    }

    /// Load with an explicit forge adapter (used in tests). The session identity
    /// is `None`, so handlers fall back to the caller-provided identity — keeping
    /// tests independent of the ambient `CLAUDE_CODE_SESSION_ID`.
    ///
    /// # Errors
    /// Returns an error if the event log cannot be read.
    pub async fn load_with_forge(
        project_root: PathBuf,
        forge: Arc<dyn ForgeAdapter>,
    ) -> anyhow::Result<Self> {
        Self::load_with_forge_and_session(project_root, forge, None).await
    }

    /// Load with an explicit forge adapter and session identity.
    ///
    /// # Errors
    /// Returns an error if the event log cannot be read.
    pub async fn load_with_forge_and_session(
        project_root: PathBuf,
        forge: Arc<dyn ForgeAdapter>,
        session_identity: Option<String>,
    ) -> anyhow::Result<Self> {
        let store_dir = eventcore_store_dir(&project_root);
        std::fs::create_dir_all(&store_dir)?;
        let event_store = FileEventStore::open(&store_dir)?;

        let project = load_project_state_v2(&event_store, &project_root).await?;
        let v2_stream_version = cfk_engine::events::stream_event_count(&event_store).await?;

        Ok(Self {
            state: Arc::new(RwLock::new(ServerState::new(
                project_root,
                project,
                event_store,
                v2_stream_version,
            ))),
            forge,
            session_identity,
            tool_router: Self::tool_router(),
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn tool_error(msg: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![Content::text(msg)])
}

fn command_error(e: &CommandError) -> McpError {
    // Routing is always a kernel misconfiguration, not a caller mistake.
    // All other variants indicate an invalid call sequence or bad params.
    let msg = e.to_string();
    if matches!(e, CommandError::Routing(_)) {
        McpError::internal_error(msg, None)
    } else {
        McpError::invalid_params(msg, None)
    }
}

fn work_item_summary(item: &WorkItem) -> WorkItemSummary {
    WorkItemSummary {
        id: item.id.to_string(),
        phase: format!("{:?}", item.phase).to_lowercase(),
        work_type: format!("{:?}", item.work_type),
        status: format!("{:?}", item.status).to_lowercase(),
        description: item.description.clone(),
    }
}

/// Compute the per-slice promotion chain heads to spawn for the current state
/// (ADR 0011). Pure given the project and the verified slices; the caller emits
/// the returned `WorkItemAdded` events. Shared by `cf_next_step` reconciliation
/// and the manual `cf_ingest_slices` backfill so both seed the chain identically
/// (a slug already in the chain yields nothing, never a phase-skipping item).
fn compute_slice_promotions(proj: &ProjectState, slices: &[EmcSlice]) -> Vec<WorkItem> {
    // Defend against a verified model emitting the same slug more than once
    // (e.g. a re-edited slice): only the first occurrence seeds the chain, so a
    // single call never spawns two heads for one slug.
    let mut seen = std::collections::HashSet::new();
    slices
        .iter()
        .filter(|slice| seen.insert(slice.slug.as_str()))
        .filter_map(|slice| {
            let existing: Vec<(PhaseKind, WorkItemStatus)> = proj
                .work_items
                .iter()
                .filter(|i| i.emc_slug.as_deref() == Some(slice.slug.as_str()))
                .map(|i| (i.phase, i.status))
                .collect();
            let dev_work_type = match slice.kind.as_str() {
                "translation" | "mechanical" => WorkType::MechanicalTransform,
                _ => WorkType::NarrowestStepImplementation,
            };
            next_slice_promotion(&existing, dev_work_type).map(|head| {
                WorkItem::from_emc_slice(
                    WorkItemId::new(),
                    head.phase,
                    head.work_type,
                    format!("[{}] {}", slice.slug, slice.name),
                    slice.slug.clone(),
                )
            })
        })
        .collect()
}

fn content_json<T>(value: &T) -> Result<CallToolResult, McpError>
where
    T: Serialize,
{
    let content = Content::json(value)?;
    Ok(CallToolResult::success(vec![content]))
}

fn parse_work_item_id(s: &str) -> Result<WorkItemId, McpError> {
    let uuid = Uuid::parse_str(s)
        .map_err(|_| McpError::invalid_params("invalid work_item_id (expected UUID)", None))?;
    WorkItemId::try_new(uuid)
        .map_err(|_| McpError::invalid_params("invalid work_item_id", None))
}

fn tdd_phase_label(phase: &TddPhase) -> &'static str {
    match phase {
        TddPhase::WriteTest => "write_test",
        TddPhase::TestReviewGate => "test_review_gate",
        TddPhase::RedCheck => "red_check",
        TddPhase::Implement => "implement",
        TddPhase::CheckProgress => "check_progress",
        TddPhase::ImplReviewGate => "impl_review_gate",
        TddPhase::LintCheck => "lint_check",
        TddPhase::Done => "done",
    }
}

// ── Tool implementations ─────────────────────────────────────────────────

#[tool_router]
impl CfkServer {
    #[tool(description = "\
Initialize `.claude-factory/` in a product repo. Must be called once before \
any other `cf_*` tool. Returns the new project ID.")]
    pub async fn cf_init(
        &self,
        Parameters(params): Parameters<InitParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut guard = self.state.write().await;

        if guard.project.is_some() {
            return Ok(tool_error("Project is already initialized in this root."));
        }

        let root = params
            .project_root
            .map_or_else(|| guard.project_root.clone(), PathBuf::from);

        let project_id = cfk_core::types::ids::ProjectId::new();
        let routing = default_routing_table();
        let project_state = ProjectState::new(project_id.clone(), root.clone(), routing);

        // Set project before emitting so apply_event in emit() can update it.
        guard.project = Some(project_state);

        guard.emit(FactoryEvent::ProjectInitialized { id: project_id.clone() })
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        // Set up project-local Claude Code settings with conductor defaults.
        let claude_dir = root.join(".claude");
        let settings_path = claude_dir.join("settings.json");

        std::fs::create_dir_all(&claude_dir)
            .map_err(|e| McpError::internal_error(format!("failed to create .claude directory: {e}"), None))?;

        // Read existing settings or start with empty object.
        let mut settings: serde_json::Value = if settings_path.exists() {
            std::fs::read_to_string(&settings_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_else(|| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        // Set conductor defaults.
        if let Some(obj) = settings.as_object_mut() {
            obj.insert("model".to_string(), serde_json::json!("haiku"));
            obj.insert("advisor".to_string(), serde_json::json!("fable"));
        }

        // Write back with formatting.
        let formatted = serde_json::to_string_pretty(&settings)
            .map_err(|e| McpError::internal_error(format!("failed to serialize settings: {e}"), None))?;
        std::fs::write(&settings_path, formatted)
            .map_err(|e| McpError::internal_error(format!("failed to write .claude/settings.json: {e}"), None))?;

        content_json(&serde_json::json!({
            "project_id": project_id.to_string(),
            "root": root.display().to_string(),
        }))
    }

    #[tool(description = "\
Return a compact dashboard of work-item counts per phase. \
Shows ready / in_progress / done / abandoned for each phase.")]
    pub async fn cf_status(
        &self,
        Parameters(_params): Parameters<PhaseFilterParams>,
    ) -> Result<CallToolResult, McpError> {
        let guard = self.state.read().await;
        let Some(ref proj) = guard.project else {
            return Ok(tool_error("Project not initialized. Run `cf_init` first."));
        };

        let counts = proj.phase_counts();
        let phases: Vec<PhaseStatusEntry> = PhaseKind::all()
            .iter()
            .map(|phase| {
                let c = counts.get(phase).copied().unwrap_or_default();
                PhaseStatusEntry {
                    phase: *phase,
                    ready: c.ready,
                    in_progress: c.in_progress,
                    done: c.done,
                    abandoned: c.abandoned,
                }
            })
            .filter(|p| p.ready > 0 || p.in_progress > 0 || p.done > 0 || p.abandoned > 0)
            .collect();

        content_json(&StatusResponse {
            project_id: proj.id.to_string(),
            phases,
        })
    }

    #[tool(description = "\
Return the next work step for the conductor to execute. For development-phase \
items, the step is TDD-phase-specific. The response `status` is `ready` or \
`idle`. When `ready`, `action.type` is one of: `spawn_agent`, `run_check`, \
`gate_review`. When `action.type` is `run_check`, call `cf_run_check`. When \
`action.type` is `gate_review`, run the reviewer agent then call `cf_gate`.")]
    pub async fn cf_next_step(
        &self,
        Parameters(params): Parameters<NextStepParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut guard = self.state.write().await;

        // ── Per-slice promotion reconciliation (ADR 0011) ──────────────────────
        // Each verified emc slice flows Architecture → DesignSystem → Development.
        // Spawn the next chain-head for any slice that needs one. Pure decision in
        // cfk_core::promotion; this shell reads the verified model and emits.
        {
            let to_spawn: Vec<WorkItem> = if let Some(ref proj) = guard.project {
                let root = guard.project_root.clone();
                let slices = read_verified_slices(&root)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                compute_slice_promotions(proj, &slices)
            } else {
                Vec::new()
            };
            for item in to_spawn {
                guard
                    .emit(FactoryEvent::WorkItemAdded { work_item: item })
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            }
        }

        let Some(ref proj) = guard.project else {
            return Ok(tool_error("Project not initialized. Run `cf_init` first."));
        };

        let phase_filter = params.phase;

        // Auto-claim ready development items under this session's identity.
        // Prefer the Claude Code session id (what the guardrail checks) over any
        // caller-provided string, so the lease matches the editing session.
        let resolved_session = self
            .session_identity
            .clone()
            .or_else(|| params.session_identity.clone());
        if let Some(ref session_id) = resolved_session {
            // Find the first ready development item (in phase order).
            let ready_dev: Option<WorkItemId> = proj
                .work_items
                .iter()
                .find(|i| {
                    i.phase == PhaseKind::Development
                        && i.status == WorkItemStatus::Ready
                        && phase_filter.is_none_or(|p| i.phase == p)
                })
                .map(|i| i.id.clone());

            if let Some(wid) = ready_dev {
                let lease = handle_claim(proj, &wid, session_id)
                    .map_err(|e| command_error(&e))?;

                guard
                    .emit(FactoryEvent::LeaseGranted { lease })
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                // Start TDD slice state.
                let author_identity = AuthorIdentity::try_new(session_id.clone())
                    .map_err(|_| McpError::invalid_params("Invalid session identity", None))?;
                guard
                    .emit(FactoryEvent::TddSliceStarted {
                        work_item_id: wid,
                        author_identity,
                    })
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            }
        }

        // Auto-complete dev items that reached TddPhase::Done and spin up a
        // paired Review work item for each so the PR flow can start immediately.
        {
            let done_dev: Vec<(WorkItemId, String)> = guard
                .project
                .as_ref()
                .map(|p| {
                    p.work_items
                        .iter()
                        .filter(|i| {
                            i.phase == PhaseKind::Development
                                && i.status == WorkItemStatus::InProgress
                                && p.dev_states
                                    .get(&i.id)
                                    .and_then(|d| d.current_phase())
                                    == Some(&TddPhase::Done)
                        })
                        .map(|i| (i.id.clone(), i.description.clone()))
                        .collect()
                })
                .unwrap_or_default();

            for (wid, description) in done_dev {
                guard
                    .emit(FactoryEvent::TddSliceDone { work_item_id: wid.clone() })
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                guard
                    .emit(FactoryEvent::WorkItemCompleted { work_item_id: wid.clone() })
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                // Create a paired Review work item for the PR flow.
                let review_item = WorkItem::new(
                    WorkItemId::new(),
                    PhaseKind::Review,
                    WorkType::PrCommentTriage,
                    description,
                );
                let review_wid = review_item.id.clone();
                guard
                    .emit(FactoryEvent::WorkItemAdded { work_item: review_item })
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                // Auto-claim the new review item under this session so the
                // conductor can immediately dispatch an OpenPr step for it.
                if let Some(ref session_id) = resolved_session {
                    let proj = guard.project.as_ref()
                        .ok_or_else(|| McpError::internal_error(
                            "project state lost after review item add", None,
                        ))?;
                    let lease = handle_claim(proj, &review_wid, session_id)
                        .map_err(|e| command_error(&e))?;
                    guard
                        .emit(FactoryEvent::LeaseGranted { lease })
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                }
            }
        }

        let proj = guard.project.as_ref()
            .ok_or_else(|| McpError::internal_error("project state lost during auto-claim", None))?;
        let response = handle_next_step(proj, phase_filter)
            .map_err(|e| command_error(&e))?;

        content_json(&serde_json::to_value(&response).map_err(|e| {
            McpError::internal_error(format!("serialization error: {e}"), None)
        })?)
    }

    #[tool(description = "\
Claim a work item for this session. Returns a `lease_id` and `granted_at` \
timestamp. The conductor must hold the lease while executing the step and \
release it on completion or failure.")]
    pub async fn cf_claim(
        &self,
        Parameters(params): Parameters<ClaimParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut guard = self.state.write().await;
        let Some(ref proj) = guard.project else {
            return Ok(tool_error("Project not initialized."));
        };

        let work_item_id = parse_work_item_id(&params.work_item_id)?;

        // Key the lease to the Claude Code session (what the guardrail checks),
        // falling back to the caller-provided identity outside Claude Code.
        let session_id = self
            .session_identity
            .clone()
            .unwrap_or_else(|| params.session_identity.clone());
        let lease = handle_claim(proj, &work_item_id, &session_id)
            .map_err(|e| command_error(&e))?;

        let lease_id_str = lease.id.to_string();
        let granted_at = lease.granted_at;

        guard
            .emit(FactoryEvent::LeaseGranted { lease })
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::json!({
            "lease_id": lease_id_str,
            "work_item_id": params.work_item_id,
            "granted_at": granted_at,
        }))
    }

    #[tool(description = "\
Release the lease on a work item (e.g. on session failure or abort). \
The item returns to `Ready` status and can be claimed again.")]
    pub async fn cf_release(
        &self,
        Parameters(params): Parameters<WorkItemIdParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut guard = self.state.write().await;
        let Some(ref proj) = guard.project else {
            return Ok(tool_error("Project not initialized."));
        };

        let work_item_id = parse_work_item_id(&params.work_item_id)?;

        let Some(item) = proj.work_items.iter().find(|i| i.id == work_item_id) else {
            return Ok(tool_error(format!(
                "work item {} not found",
                params.work_item_id
            )));
        };

        if item.status != WorkItemStatus::InProgress {
            return Ok(tool_error(format!(
                "work item {} is not in progress",
                params.work_item_id
            )));
        }

        let Some(lease_id) = item.active_lease.clone() else {
            return Ok(tool_error(format!(
                "work item {} has no active lease",
                params.work_item_id
            )));
        };

        guard
            .emit(FactoryEvent::LeaseReleased { lease_id, work_item_id })
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::json!({ "released": params.work_item_id }))
    }

    #[tool(description = "\
Submit the result of a step, advancing the TDD state machine.\n\n\
For `WriteTest` phase: `result` must be `{\"test_content\": \"<code>\"}`.\n\
For `Implement` phase: `result` may include `{\"drill_down_description\": \"<desc>\"}`\n\
if the error requires a tighter unit test; omit or set to null otherwise.\n\
For other phases: any JSON value is accepted as evidence.")]
    pub async fn cf_submit(
        &self,
        Parameters(params): Parameters<SubmitParams>,
    ) -> Result<CallToolResult, McpError> {
        let work_item_id = parse_work_item_id(&params.work_item_id)?;

        // Parse JSON → SubmissionPayload at the boundary (read lock).
        let payload = {
            let guard = self.state.read().await;
            let proj = guard.project.as_ref()
                .ok_or_else(|| McpError::invalid_params("Project not initialized.", None))?;

            if !proj.work_items.iter().any(|i| i.id == work_item_id) {
                return Ok(tool_error(format!("work item {} not found", params.work_item_id)));
            }

            let effective_session: Option<SessionIdentity> = self
                .session_identity
                .as_deref()
                .and_then(|s| SessionIdentity::try_new(s).ok());
            let now = chrono::Utc::now();
            let lease_valid = proj.leases.iter().any(|l| {
                l.work_item_id == work_item_id
                    && effective_session
                        .as_ref()
                        .is_none_or(|s| &l.session_identity == s)
                    && !l.is_expired(now)
            });
            if !lease_valid {
                return Ok(tool_error(format!(
                    "no active lease for work item {} — claim the item before submitting",
                    params.work_item_id
                )));
            }

            let current_tdd_phase = proj.dev_states.get(&work_item_id)
                .and_then(|d| d.current_phase()).cloned();
            let item_work_type = proj.work_items.iter()
                .find(|i| i.id == work_item_id)
                .map(|i| i.work_type);

            // Some MCP conductors JSON-encode the structured `result` argument as
            // a string; unwrap it back to an object so field lookups succeed.
            let result = normalize_submission_result(params.result.clone());

            match current_tdd_phase {
                Some(TddPhase::WriteTest) => {
                    let raw = result.get("test_content")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| McpError::invalid_params(
                            "WriteTest submission must include {\"test_content\": \"...\"}",
                            None,
                        ))?.to_string();
                    let test_content = TestCode::try_new(raw)
                        .map_err(|_| McpError::invalid_params("test_content cannot be empty", None))?;
                    SubmissionPayload::Test { test_content }
                }
                Some(TddPhase::Implement) => {
                    let drill_down = result.get("drill_down_description")
                        .and_then(serde_json::Value::as_str)
                        .map(|s| DrillDownDescription::try_new(s.to_string())
                            .map_err(|_| McpError::invalid_params(
                                "drill_down_description cannot be empty", None,
                            )))
                        .transpose()?;
                    SubmissionPayload::Implementation { drill_down }
                }
                None if item_work_type == Some(WorkType::PrCommentTriage) => {
                    let raw = result.get("reply")
                        .and_then(serde_json::Value::as_str)
                        .or_else(|| result.as_str())
                        .unwrap_or("(no reply provided)")
                        .to_string();
                    let reply = CommentBody::try_new(raw)
                        .map_err(|_| McpError::invalid_params("reply cannot be empty", None))?;
                    SubmissionPayload::TriageReply { reply }
                }
                Some(_) | None => SubmissionPayload::Generic(result),
            }
        };

        // Delegate state-machine logic to the engine (pure, read-only state).
        let (events, outcome) = {
            let guard = self.state.read().await;
            let proj = guard.project.as_ref()
                .ok_or_else(|| McpError::invalid_params("Project not initialized.", None))?;
            handle_submission(proj, work_item_id.clone(), payload)
                .map_err(|ref e| command_error(e))?
        };

        // Perform forge side-effects before committing events.
        if let SubmissionOutcome::CommentQueued { pr_number, ref reply } = outcome {
            self.forge
                .post_comment(pr_number, reply)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        // Append events.
        let mut guard = self.state.write().await;
        for event in events {
            guard.emit(event).await.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        // Serialize outcome.
        let response = match outcome {
            SubmissionOutcome::AdvancedTo(ref phase) => serde_json::json!({
                "work_item_id": params.work_item_id,
                "advanced_to": tdd_phase_label(phase),
            }),
            SubmissionOutcome::DrillDownPushed { depth } => serde_json::json!({
                "work_item_id": params.work_item_id,
                "advanced_to": "write_test",
                "drill_down_depth": depth,
            }),
            SubmissionOutcome::Acknowledged => serde_json::json!({
                "work_item_id": params.work_item_id,
                "status": "acknowledged",
            }),
            SubmissionOutcome::CommentQueued { .. } => serde_json::json!({
                "completed": params.work_item_id,
            }),
        };

        content_json(&response)
    }

    #[tool(description = "\
Record a gate verdict (test review, implementation review, or ADR review). \
The reviewer identity must differ from the work item's claiming session. \
For TDD gates: when vetoed, the cycle loops back; when approved, it advances. \
For ADR review: `approved` accepts the ADR into the registry; `vetoed` rejects it. \
`reason` is required when vetoed.")]
    pub async fn cf_gate(
        &self,
        Parameters(params): Parameters<GateParams>,
    ) -> Result<CallToolResult, McpError> {
        let work_item_id = parse_work_item_id(&params.work_item_id)?;

        // Determine gate kind and validate reviewer before taking write lock.
        let (gate_kind, reviewer_ok, is_adr_gate, adr_id_and_root) = {
            let guard = self.state.read().await;
            let proj = guard.project.as_ref()
                .ok_or_else(|| McpError::invalid_params("Project not initialized.", None))?;

            // Check TDD gate first.
            let tdd_phase = proj.dev_states.get(&work_item_id)
                .and_then(|d| d.current_phase()).cloned();

            let (gk, is_adr) = match tdd_phase.as_ref() {
                Some(TddPhase::TestReviewGate) => (GateKind::TestReview, false),
                Some(TddPhase::ImplReviewGate) => (GateKind::ImplementationReview, false),
                _ => {
                    // Check if it's an ADR gate.
                    let adr_phase = proj.adr_states.get(&work_item_id).map(|a| &a.phase);
                    if adr_phase == Some(&AdrPhase::PendingReview) {
                        (GateKind::AdrReview, true)
                    } else {
                        return Ok(tool_error(format!(
                            "work item {} is not at a gate (TDD phase: {:?}, ADR phase: {:?})",
                            params.work_item_id,
                            tdd_phase,
                            adr_phase,
                        )));
                    }
                }
            };

            let adr_info = if is_adr {
                let adr_id = proj.adr_states.get(&work_item_id)
                    .and_then(|a| a.adr_id.clone());
                Some((adr_id, guard.project_root.clone()))
            } else {
                None
            };

            let ok = validate_gate_verdict(
                proj, &work_item_id, &params.reviewer_id, gk, &GateVerdict::Approved,
            );
            (gk, ok, is_adr, adr_info)
        };

        reviewer_ok.map_err(|e| command_error(&e))?;

        let verdict = match params.verdict.as_str() {
            "approved" => GateVerdict::Approved,
            "vetoed" => {
                let raw_reason = params.reason.clone().unwrap_or_default();
                let reason = VetoReason::try_new(raw_reason).map_err(|_| {
                    McpError::invalid_params(
                        "vetoed verdict requires a non-empty `reason`",
                        None,
                    )
                })?;
                GateVerdict::Vetoed { reason }
            }
            other => {
                return Ok(tool_error(format!(
                    "invalid verdict '{other}': expected 'approved' or 'vetoed'"
                )));
            }
        };

        if is_adr_gate {
            let accepted = verdict.is_approved();
            let (adr_id_opt, project_root) = adr_id_and_root
                .ok_or_else(|| McpError::internal_error("ADR gate has no adr_info", None))?;
            let adr_id = adr_id_opt.ok_or_else(|| {
                McpError::internal_error("ADR has no ID yet", None)
            })?;

            let reason = match &verdict {
                GateVerdict::Vetoed { reason } => Some(reason.clone().into_inner()),
                GateVerdict::Approved => None,
            };

            let mut guard = self.state.write().await;
            guard.emit(FactoryEvent::AdrDecided {
                work_item_id: work_item_id.clone(),
                adr_id: adr_id.clone(),
                accepted,
                reason,
            }).await.map_err(|e| McpError::internal_error(e.to_string(), None))?;

            // Only complete the work item when the ADR is accepted. On veto the item
            // transitions to PendingHumanDecision — cf_next_step will surface an
            // ask_human action so a human can decide to requeue, escalate, or abandon.
            if accepted {
                guard.emit(FactoryEvent::WorkItemCompleted {
                    work_item_id: work_item_id.clone(),
                }).await.map_err(|e| McpError::internal_error(e.to_string(), None))?;
            }

            // Project ARCHITECTURE.md when an ADR is accepted.
            if accepted && let Some(ref proj) = guard.project {
                project_architecture_md(&project_root, &proj.adrs)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            }

            return content_json(&serde_json::json!({
                "work_item_id": params.work_item_id,
                "adr_id": adr_id.to_string(),
                "accepted": accepted,
            }));
        }

        // TDD gate path.
        let new_phase = match (&gate_kind, &verdict) {
            (GateKind::TestReview, GateVerdict::Approved) => TddPhase::RedCheck,
            (GateKind::TestReview, GateVerdict::Vetoed { .. }) => TddPhase::WriteTest,
            (GateKind::ImplementationReview, GateVerdict::Approved) => TddPhase::LintCheck,
            (GateKind::ImplementationReview, GateVerdict::Vetoed { .. }) => TddPhase::Implement,
            // AdrReview exits early above; this arm is a compile-time exhaustiveness placeholder.
            (GateKind::AdrReview, _) => {
                return Ok(tool_error("unexpected ADR gate kind in TDD gate path".to_string()));
            }
        };

        let new_phase_label = tdd_phase_label(&new_phase);

        let reviewer_id = ReviewerId::try_new(params.reviewer_id.clone())
            .map_err(|_| McpError::invalid_params("reviewer_id cannot be empty", None))?;
        let mut guard = self.state.write().await;
        guard
            .emit(FactoryEvent::TddGateVerdict {
                work_item_id: work_item_id.clone(),
                gate_kind,
                verdict,
                reviewer_id,
            })
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::json!({
            "work_item_id": params.work_item_id,
            "reviewer_id": params.reviewer_id,
            "advanced_to": new_phase_label,
        }))
    }

    #[tool(description = "\
List all work items in the backlog, optionally filtered by phase.")]
    pub async fn cf_backlog(
        &self,
        Parameters(params): Parameters<PhaseFilterParams>,
    ) -> Result<CallToolResult, McpError> {
        let guard = self.state.read().await;
        let Some(ref proj) = guard.project else {
            return Ok(tool_error("Project not initialized."));
        };

        let phase_filter = params.phase;

        let items: Vec<WorkItemSummary> = proj
            .work_items
            .iter()
            .filter(|i| phase_filter.is_none_or(|p| i.phase == p))
            .map(work_item_summary)
            .collect();

        content_json(&serde_json::json!({ "items": items }))
    }

    #[tool(description = "\
Add a work item to the backlog. \
`phase`: `discovery` | `event_modeling` | `architecture` | `design_system` \
| `development` | `review`. \
`work_type`: `socratic_discovery` | `event_model_authoring` | `adr_drafting` \
| `outer_behavioral_test_writing` | `test_review` | \
`narrowest_step_implementation` | `implementation_review` | \
`mechanical_transform` | `pr_comment_triage` | `research`.")]
    pub async fn cf_backlog_add(
        &self,
        Parameters(params): Parameters<BacklogAddParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut guard = self.state.write().await;
        if guard.project.is_none() {
            return Ok(tool_error("Project not initialized."));
        }

        let item = WorkItem::new(
            cfk_core::types::ids::WorkItemId::new(),
            params.phase,
            params.work_type,
            params.description,
        );

        let item_id = item.id.to_string();

        guard
            .emit(FactoryEvent::WorkItemAdded { work_item: item })
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::json!({ "work_item_id": item_id }))
    }

    #[tool(description = "Show the current routing table (work type → executor mapping).")]
    pub async fn cf_route(
        &self,
        Parameters(_params): Parameters<PhaseFilterParams>,
    ) -> Result<CallToolResult, McpError> {
        let guard = self.state.read().await;
        let routing = guard
            .project
            .as_ref()
            .map_or_else(default_routing_table, |p| p.routing.clone());

        content_json(&serde_json::to_value(&routing).map_err(|e| {
            McpError::internal_error(format!("serialization error: {e}"), None)
        })?)
    }

    #[tool(description = "\
Manually reconcile the per-slice promotion chain (ADR 0011) from a formally-\
verified emc model — a backlog/backfill convenience; `cf_next_step` performs the \
same reconciliation automatically. Reads `model/events/v1/` under the product \
repo, finds all `SliceAdded` events whose workflow has a \
`WorkflowReadinessDeclared` event, and for each slice spawns the SINGLE next \
chain-head work item it needs (a new slice → an ArchitectureTriage gate; never a \
phase-skipping Development item). Idempotent: slices already as far along as their \
items allow yield nothing. Returns counts of spawned and unchanged slices.")]
    pub async fn cf_ingest_slices(
        &self,
        Parameters(params): Parameters<IngestSlicesParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut guard = self.state.write().await;
        let Some(ref proj) = guard.project else {
            return Ok(tool_error("Project not initialized. Run `cf_init` first."));
        };

        let root = params
            .project_root
            .map_or_else(|| guard.project_root.clone(), PathBuf::from);

        let slices = read_verified_slices(&root)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let total_slices = slices.len();

        let to_spawn = compute_slice_promotions(proj, &slices);
        let spawned = to_spawn.len();
        let mut new_ids: Vec<String> = Vec::with_capacity(spawned);

        for item in to_spawn {
            new_ids.push(item.id.to_string());
            guard
                .emit(FactoryEvent::WorkItemAdded { work_item: item })
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        content_json(&serde_json::json!({
            "spawned": spawned,
            "unchanged": total_slices.saturating_sub(spawned),
            "work_item_ids": new_ids,
        }))
    }

    #[tool(description = "\
Run a configured deterministic check (tests, linter, build) and record the \
result as evidence. Agents never self-report pass/fail — the kernel always \
runs checks itself.\n\n\
Provide `work_item_id` to advance the TDD state machine based on the result.")]
    pub async fn cf_run_check(
        &self,
        Parameters(params): Parameters<RunCheckParams>,
    ) -> Result<CallToolResult, McpError> {
        let project_root = {
            let guard = self.state.read().await;
            guard.project_root.clone()
        };

        let checks = load_checks(&project_root)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let command = checks.command_for(&params.check_name).ok_or_else(|| {
            McpError::invalid_params(
                format!("unknown check '{}' — configure it in .claude-factory/checks.toml or use a built-in (tests, lint, build)", params.check_name),
                None,
            )
        })?;

        let result = run_check(&command, &project_root)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        // Optionally advance TDD state machine.
        if let Some(wid_str) = &params.work_item_id {
            let work_item_id = parse_work_item_id(wid_str)?;

            let check_name = CheckName::try_new(params.check_name.clone())
                .map_err(|_| McpError::invalid_params("check_name cannot be empty", None))?;
            let first_error = result.first_error.clone()
                .map(ErrorMessage::try_new)
                .transpose()
                .map_err(|_| McpError::internal_error("Invalid error message".to_string(), None))?;
            let mut guard = self.state.write().await;
            guard
                .emit(FactoryEvent::TddCheckResult {
                    work_item_id,
                    check_name,
                    passed: result.passed,
                    first_error,
                })
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        let tdd_phase = {
            let guard = self.state.read().await;
            params.work_item_id.as_deref()
                .and_then(|id| Uuid::parse_str(id).ok())
                .and_then(|uuid| WorkItemId::try_new(uuid).ok())
                .and_then(|wid| {
                    guard.project.as_ref()?.dev_states.get(&wid)?.current_phase().map(tdd_phase_label)
                })
        };

        let mut resp = serde_json::json!({
            "check_name": params.check_name,
            "passed": result.passed,
            "first_error": result.first_error,
        });
        if let Some(phase) = tdd_phase
            && let Some(obj) = resp.as_object_mut()
        {
            obj.insert("advanced_tdd_phase_to".to_string(), serde_json::Value::String(phase.to_string()));
        }

        content_json(&resp)
    }

    #[tool(description = "\
Open a pull request on the forge for a review-phase slice. \
Returns the PR number and URL. `head` is the source branch; `base` is the \
target branch (usually `main`).")]
    pub async fn cf_pr_open(
        &self,
        Parameters(params): Parameters<PrOpenParams>,
    ) -> Result<CallToolResult, McpError> {
        let work_item_id = parse_work_item_id(&params.work_item_id)?;

        // Validate before hitting the forge (release lock before await).
        {
            let guard = self.state.read().await;
            let proj = guard.project.as_ref()
                .ok_or_else(|| McpError::invalid_params("Project not initialized.", None))?;
            if !proj.work_items.iter().any(|i| i.id == work_item_id) {
                return Ok(tool_error(format!("work item {} not found", params.work_item_id)));
            }
            if let Some(review) = proj.review_states.get(&work_item_id) {
                use cfk_core::state_machine::review::ReviewSlicePhase;
                if review.phase != ReviewSlicePhase::WaitingForPr {
                    return Ok(tool_error(format!(
                        "work item {} is not in WaitingForPr phase", params.work_item_id
                    )));
                }
            }
        } // read lock released here

        // Snapshot the state we need, then call forge (async, no lock held).
        let state_clone = {
            let guard = self.state.read().await;
            guard.project.as_ref()
                .ok_or_else(|| McpError::invalid_params("Project not initialized.", None))
                .map(|p| {
                    // Build a minimal snapshot: just the work item and review state.
                    // We use handle_pr_open which needs the full state — clone it.
                    // ProjectState doesn't impl Clone, so call handle_pr_open directly
                    // from the guard, which is fine because handle_pr_open doesn't await
                    // on anything that needs a lock — we release the guard before the forge call.
                    (
                        p.work_items.iter().find(|i| i.id == work_item_id).map(|i| i.id.clone()),
                        p.review_states.get(&work_item_id).map(|r| r.phase.clone()),
                    )
                })?
        };
        let _ = state_clone; // already validated above

        // Build PR spec and open via forge (no lock held).
        let spec = cfk_engine::forge::PrSpec {
            title: params.title.clone(),
            body: params.body.clone(),
            head: params.head.clone(),
            base: params.base.clone(),
        };
        let opened = self.forge.open_pr(&spec).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let pr_number_raw = opened.number;
        let pr_url_raw = opened.url.clone();
        let pr_url_typed = PrUrl::try_new(pr_url_raw.clone())
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut guard = self.state.write().await;
        guard.emit(FactoryEvent::ReviewSliceStarted {
            work_item_id,
            pr_number: PrNumber::new(pr_number_raw),
            pr_url: pr_url_typed,
        }).await.map_err(|e| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::json!({
            "work_item_id": params.work_item_id,
            "pr_number": pr_number_raw,
            "pr_url": pr_url_raw,
        }))
    }

    #[tool(description = "\
Poll the forge for a review-phase PR: CI status, review approvals, and new \
comments. New comments produce `PrCommentTriage` work items. \
Returns a summary of CI status and any new triage items created.")]
    pub async fn cf_pr_poll(
        &self,
        Parameters(params): Parameters<PrPollParams>,
    ) -> Result<CallToolResult, McpError> {
        let work_item_id = parse_work_item_id(&params.work_item_id)?;

        let events = {
            let guard = self.state.read().await;
            let proj = guard.project.as_ref()
                .ok_or_else(|| McpError::invalid_params("Project not initialized.", None))?;

            handle_pr_poll(proj, &work_item_id, &self.forge)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        };

        let mut triage_ids: Vec<String> = Vec::new();
        let mut all_green = false;

        for event in &events {
            match event {
                FactoryEvent::ReviewCommentTriageCreated { triage_item_id, comment_id, comment_body, .. } => {
                    // Create a PrCommentTriage work item in the backlog.
                    let body_str = comment_body.to_string();
                    let body_preview = &body_str[..body_str.floor_char_boundary(120)];
                    let triage_item = cfk_core::state_machine::work_item::WorkItem::new(
                        triage_item_id.clone(),
                        cfk_core::types::phase::PhaseKind::Review,
                        WorkType::PrCommentTriage,
                        format!("Comment {comment_id}: {body_preview}"),
                    );
                    triage_ids.push(triage_item_id.to_string());
                    let mut guard = self.state.write().await;
                    guard.emit(FactoryEvent::WorkItemAdded { work_item: triage_item })
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    guard.emit(event.clone())
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                }
                FactoryEvent::ReviewAllGreen { .. } => {
                    all_green = true;
                    let mut guard = self.state.write().await;
                    guard.emit(event.clone())
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                }
                _ => {}
            }
        }

        content_json(&serde_json::json!({
            "work_item_id": params.work_item_id,
            "all_green": all_green,
            "new_triage_items": triage_ids,
        }))
    }

    #[tool(description = "\
Merge the pull request for a review-phase slice. Only valid when the slice is \
in `all_green` state (CI passing + approved). Marks the slice as done.")]
    pub async fn cf_pr_merge(
        &self,
        Parameters(params): Parameters<PrMergeParams>,
    ) -> Result<CallToolResult, McpError> {
        let work_item_id = parse_work_item_id(&params.work_item_id)?;

        let events = {
            let guard = self.state.read().await;
            let proj = guard.project.as_ref()
                .ok_or_else(|| McpError::invalid_params("Project not initialized.", None))?;

            handle_pr_merge(proj, &work_item_id, &self.forge)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        };

        let mut guard = self.state.write().await;
        for event in events {
            guard.emit(event).await.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        content_json(&serde_json::json!({
            "work_item_id": params.work_item_id,
            "merged": true,
        }))
    }

    // ── Discovery phase tools ─────────────────────────────────────────────

    #[tool(description = "\
Submit a discovery brief and workflow list (called by the discovery agent). \
`brief_content` is the product brief; `workflows` is the list of workflow \
names to queue for event modeling on human approval.")]
    pub async fn cf_discovery_submit(
        &self,
        Parameters(params): Parameters<DiscoverySubmitParams>,
    ) -> Result<CallToolResult, McpError> {
        let work_item_id = parse_work_item_id(&params.work_item_id)?;

        {
            let guard = self.state.read().await;
            let proj = guard.project.as_ref()
                .ok_or_else(|| McpError::invalid_params("Project not initialized.", None))?;
            if !proj.work_items.iter().any(|i| i.id == work_item_id) {
                return Ok(tool_error(format!("work item {} not found", params.work_item_id)));
            }
        }

        if params.brief_content.trim().is_empty() {
            return Ok(tool_error("brief_content must not be empty"));
        }
        if params.workflows.is_empty() {
            return Ok(tool_error("workflows must not be empty"));
        }

        let mut guard = self.state.write().await;
        guard.emit(FactoryEvent::DiscoveryBriefDrafted {
            work_item_id,
            brief_content: params.brief_content.clone(),
            workflows: params.workflows.clone(),
        }).await.map_err(|e| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::json!({
            "work_item_id": params.work_item_id,
            "workflows": params.workflows,
        }))
    }

    #[tool(description = "\
Approve or reject a discovery brief (human gate). If approved, all submitted \
workflows are queued as event-modeling work items and the discovery work item \
is completed. If rejected, the discovery dialogue resets for a re-run.")]
    pub async fn cf_discovery_approve(
        &self,
        Parameters(params): Parameters<DiscoveryApproveParams>,
    ) -> Result<CallToolResult, McpError> {
        let work_item_id = parse_work_item_id(&params.work_item_id)?;

        let workflows = {
            let guard = self.state.read().await;
            let proj = guard.project.as_ref()
                .ok_or_else(|| McpError::invalid_params("Project not initialized.", None))?;

            let disc = proj.discovery_states.get(&work_item_id).ok_or_else(|| {
                McpError::invalid_params(
                    format!("work item {} has no discovery brief yet", params.work_item_id),
                    None,
                )
            })?;

            if disc.phase != DiscoveryPhase::BriefReady {
                return Ok(tool_error(format!(
                    "work item {} is not in BriefReady phase (current: {:?})",
                    params.work_item_id, disc.phase,
                )));
            }

            disc.workflows.clone()
        };

        let mut guard = self.state.write().await;
        guard.emit(FactoryEvent::DiscoveryApproved {
            work_item_id: work_item_id.clone(),
        }).await.map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut queued_ids: Vec<String> = Vec::new();

        if params.approved {
            for workflow_name in &workflows {
                let item = WorkItem::new(
                    WorkItemId::new(),
                    PhaseKind::EventModeling,
                    WorkType::EventModelAuthoring,
                    format!("Model workflow: {workflow_name}"),
                );
                queued_ids.push(item.id.to_string());
                guard.emit(FactoryEvent::WorkItemAdded { work_item: item })
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            }

            guard.emit(FactoryEvent::WorkItemCompleted {
                work_item_id: work_item_id.clone(),
            }).await.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        content_json(&serde_json::json!({
            "work_item_id": params.work_item_id,
            "approved": params.approved,
            "queued_event_modeling_items": queued_ids,
        }))
    }

    // ── Architecture phase tools ──────────────────────────────────────────

    #[tool(description = "\
Submit an ADR draft (called by the architect agent). `title` is a short \
decision title; `content` follows Context / Decision / Consequences format. \
Returns the assigned ADR ID. After submission the ADR enters review gate.")]
    pub async fn cf_adr_submit(
        &self,
        Parameters(params): Parameters<AdrSubmitParams>,
    ) -> Result<CallToolResult, McpError> {
        let work_item_id = parse_work_item_id(&params.work_item_id)?;

        {
            let guard = self.state.read().await;
            let proj = guard.project.as_ref()
                .ok_or_else(|| McpError::invalid_params("Project not initialized.", None))?;
            let Some(item) = proj.work_items.iter().find(|i| i.id == work_item_id) else {
                return Ok(tool_error(format!("work item {} not found", params.work_item_id)));
            };
            if item.status == WorkItemStatus::Done {
                return Ok(tool_error(format!(
                    "work item {} is already Done; cf_adr_submit is not allowed on completed work items",
                    params.work_item_id
                )));
            }
        }

        if params.title.trim().is_empty() {
            return Ok(tool_error("title must not be empty"));
        }
        if params.content.trim().is_empty() {
            return Ok(tool_error("content must not be empty"));
        }

        let adr_id = AdrId::new();

        let mut guard = self.state.write().await;
        guard.emit(FactoryEvent::AdrDrafted {
            work_item_id,
            adr_id: adr_id.clone(),
            title: params.title.clone(),
            content: params.content.clone(),
        }).await.map_err(|e| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::json!({
            "work_item_id": params.work_item_id,
            "adr_id": adr_id.to_string(),
            "title": params.title,
        }))
    }

    #[tool(description = "\
Submit a per-slice triage decision (ADR 0011) for an ArchitectureTriage or \
DesignTriage work item. The triage item is completed. When `needs_followup` is \
true, a follow-up work item is spawned carrying the same slice slug — an \
AdrDrafting item for architecture triage, or a DesignSystemBuild item for design \
triage — so the slice's chain waits for that work before advancing.")]
    pub async fn cf_triage_submit(
        &self,
        Parameters(params): Parameters<TriageSubmitParams>,
    ) -> Result<CallToolResult, McpError> {
        let work_item_id = parse_work_item_id(&params.work_item_id)?;

        if params.rationale.trim().is_empty() {
            return Ok(tool_error("rationale must not be empty"));
        }

        // Validate the item and capture what the follow-up needs, releasing the
        // read borrow before emitting.
        let followup: Option<WorkItem> = {
            let guard = self.state.read().await;
            let proj = guard.project.as_ref().ok_or_else(|| {
                McpError::invalid_params("Project not initialized.", None)
            })?;
            let Some(item) = proj.work_items.iter().find(|i| i.id == work_item_id) else {
                return Ok(tool_error(format!("work item {} not found", params.work_item_id)));
            };
            if matches!(item.status, WorkItemStatus::Done | WorkItemStatus::Abandoned) {
                return Ok(tool_error(format!(
                    "work item {} is already {:?}; cannot submit a triage decision",
                    params.work_item_id, item.status
                )));
            }
            let (followup_phase, followup_type) = match item.work_type {
                WorkType::ArchitectureTriage => (PhaseKind::Architecture, WorkType::AdrDrafting),
                WorkType::DesignTriage => (PhaseKind::DesignSystem, WorkType::DesignSystemBuild),
                other => {
                    return Ok(tool_error(format!(
                        "work item {} is not a triage item (work_type {other:?})",
                        params.work_item_id
                    )));
                }
            };

            if params.needs_followup {
                let id = WorkItemId::new();
                Some(item.emc_slug.clone().map_or_else(
                    || WorkItem::new(id.clone(), followup_phase, followup_type, item.description.clone()),
                    |slug| {
                        WorkItem::from_emc_slice(
                            id.clone(),
                            followup_phase,
                            followup_type,
                            item.description.clone(),
                            slug,
                        )
                    },
                ))
            } else {
                None
            }
        };

        let mut guard = self.state.write().await;
        guard
            .emit(FactoryEvent::TriageDecided {
                work_item_id: work_item_id.clone(),
                needs_followup: params.needs_followup,
                rationale: params.rationale.clone(),
            })
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        guard
            .emit(FactoryEvent::WorkItemCompleted { work_item_id })
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let followup_id = if let Some(item) = followup {
            let id = item.id.to_string();
            guard
                .emit(FactoryEvent::WorkItemAdded { work_item: item })
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            Some(id)
        } else {
            None
        };

        content_json(&serde_json::json!({
            "work_item_id": params.work_item_id,
            "needs_followup": params.needs_followup,
            "followup_work_item_id": followup_id,
        }))
    }

    // ── Design-system phase tools ─────────────────────────────────────────

    #[tool(description = "\
Add a design component to the Atomic Design inventory (called by the \
design-system agent). Marks the work item as done. \
`kind`: `quark` | `atom` | `molecule` | `organism` | `template` | `page`.")]
    pub async fn cf_design_add_component(
        &self,
        Parameters(params): Parameters<DesignAddComponentParams>,
    ) -> Result<CallToolResult, McpError> {
        let work_item_id = parse_work_item_id(&params.work_item_id)?;

        {
            let guard = self.state.read().await;
            let proj = guard.project.as_ref()
                .ok_or_else(|| McpError::invalid_params("Project not initialized.", None))?;
            if !proj.work_items.iter().any(|i| i.id == work_item_id) {
                return Ok(tool_error(format!("work item {} not found", params.work_item_id)));
            }
        }

        if params.name.trim().is_empty() {
            return Ok(tool_error("name must not be empty"));
        }

        let kind = match params.kind.as_str() {
            "quark" => AtomicKind::Quark,
            "atom" => AtomicKind::Atom,
            "molecule" => AtomicKind::Molecule,
            "organism" => AtomicKind::Organism,
            "template" => AtomicKind::Template,
            "page" => AtomicKind::Page,
            other => return Ok(tool_error(format!(
                "unknown kind '{other}': expected quark | atom | molecule | organism | template | page"
            ))),
        };

        let ownership = match params.ownership.as_deref() {
            None | Some("slice") => ComponentOwnership::Slice,
            Some("platform") => ComponentOwnership::Platform,
            Some(other) => return Ok(tool_error(format!(
                "unknown ownership '{other}': expected platform | slice"
            ))),
        };

        let component_id = ComponentId::new();

        let mut guard = self.state.write().await;
        guard.emit(FactoryEvent::DesignComponentAdded {
            work_item_id: work_item_id.clone(),
            component_id: component_id.clone(),
            name: params.name.clone(),
            kind,
            ownership,
            slice_ref: params.slice_ref.clone(),
        }).await.map_err(|e| McpError::internal_error(e.to_string(), None))?;

        guard.emit(FactoryEvent::WorkItemCompleted {
            work_item_id: work_item_id.clone(),
        }).await.map_err(|e| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::json!({
            "work_item_id": params.work_item_id,
            "component_id": component_id.to_string(),
            "name": params.name,
            "kind": params.kind,
        }))
    }

    #[tool(description = "\
Run the design-system cross-check: for each named workflow, create a \
`design_system_build` work item for any component not yet in the inventory. \
The cross-check is deterministic — it only generates items for gaps. \
Returns the IDs of any new work items created.")]
    pub async fn cf_design_cross_check(
        &self,
        Parameters(params): Parameters<DesignCrossCheckParams>,
    ) -> Result<CallToolResult, McpError> {
        let existing_names: std::collections::HashSet<String> = {
            let guard = self.state.read().await;
            let proj = guard.project.as_ref()
                .ok_or_else(|| McpError::invalid_params("Project not initialized.", None))?;
            proj.design_inventory.iter().map(|c| c.name.to_string()).collect()
        };

        let mut new_item_ids: Vec<String> = Vec::new();
        let mut guard = self.state.write().await;

        for workflow in &params.workflows {
            let component_name = format!("{workflow} Page");
            if !existing_names.contains(&component_name) {
                let item = WorkItem::new(
                    WorkItemId::new(),
                    PhaseKind::DesignSystem,
                    WorkType::DesignSystemBuild,
                    format!("Design component for workflow: {workflow}"),
                );
                let id_str = item.id.to_string();
                guard.emit(FactoryEvent::WorkItemAdded { work_item: item })
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                new_item_ids.push(id_str);
            }
        }

        if !new_item_ids.is_empty() {
            guard.emit(FactoryEvent::DesignCrossCheckCompleted {
                generated_item_ids: new_item_ids.iter()
                    .filter_map(|s| {
                        Uuid::parse_str(s).ok()
                            .and_then(|u| WorkItemId::try_new(u).ok())
                    })
                    .collect(),
            }).await.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        content_json(&serde_json::json!({
            "new_work_item_ids": new_item_ids,
            "gap_count": new_item_ids.len(),
        }))
    }

    #[tool(description = "\
Mark a work item as Abandoned. The item must exist in the project. \
Once abandoned it no longer appears in the ready/in-progress counts and \
increments the abandoned counter visible through `cf_status`.")]
    pub async fn cf_abandon(
        &self,
        Parameters(params): Parameters<AbandonParams>,
    ) -> Result<CallToolResult, McpError> {
        let work_item_id = parse_work_item_id(&params.work_item_id)?;

        {
            let guard = self.state.read().await;
            let Some(ref proj) = guard.project else {
                return Ok(tool_error("Project not initialized."));
            };
            let Some(item) = proj.work_items.iter().find(|i| i.id == work_item_id) else {
                return Ok(tool_error(format!(
                    "work item {} not found",
                    params.work_item_id
                )));
            };
            if item.status == WorkItemStatus::Abandoned || item.status == WorkItemStatus::Done {
                return Ok(tool_error(format!(
                    "work item {} is already {}; cannot abandon a terminal work item",
                    params.work_item_id,
                    format!("{:?}", item.status).to_lowercase(),
                )));
            }
            if item.status == WorkItemStatus::InProgress {
                return Ok(tool_error(format!(
                    "work item {} is in-progress; call cf_release before abandoning",
                    params.work_item_id
                )));
            }
        }

        let mut guard = self.state.write().await;
        guard
            .emit(FactoryEvent::WorkItemAbandoned { work_item_id })
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::json!({ "abandoned": params.work_item_id }))
    }

    #[tool(description = "\
Return aggregated per-work-type metrics: veto rates and average token costs. \
Use this to justify routing table defaults and identify work types where the \
current executor is under-performing (high veto rate = consider a stronger \
model or a different provider). Returns entries sorted by veto rate descending.")]
    pub async fn cf_metrics(
        &self,
    ) -> Result<CallToolResult, McpError> {
        let guard = self.state.read().await;
        let proj = guard.project.as_ref()
            .ok_or_else(|| McpError::invalid_params("Project not initialized.", None))?;

        let summary = handle_metrics(proj);
        content_json(&summary)
    }

    #[tool(description = "\
Record the outcome of a completed step for routing-table metrics. \
Call this after every gate verdict (outcome: approved/vetoed) and after \
every non-gate step completion (outcome: completed). \
Provide `tokens_used` when the agent reports it. \
The kernel accumulates these outcomes to compute per-work-type veto rates \
and average token costs, which justify and guide routing table tuning.")]
    pub async fn cf_record_outcome(
        &self,
        Parameters(params): Parameters<RecordOutcomeParams>,
    ) -> Result<CallToolResult, McpError> {
        let work_item_id = parse_work_item_id(&params.work_item_id)?;

        let outcome = match params.outcome.as_str() {
            "approved" => StepOutcome::Approved,
            "vetoed" => StepOutcome::Vetoed,
            "completed" => StepOutcome::Completed,
            other => return Ok(tool_error(format!(
                "unknown outcome '{other}' — must be 'approved', 'vetoed', or 'completed'"
            ))),
        };

        let mut guard = self.state.write().await;
        let proj = guard.project.as_ref()
            .ok_or_else(|| McpError::invalid_params("Project not initialized.", None))?;

        let event = handle_record_outcome(proj, &work_item_id, outcome, params.tokens_used)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        guard.emit(event)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::json!({ "recorded": true }))
    }
}

#[tool_handler]
impl ServerHandler for CfkServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}
