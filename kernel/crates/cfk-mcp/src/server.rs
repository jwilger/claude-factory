//! MCP server implementation — exposes `cf_*` tools over stdio.

use cfk_core::{
    state_machine::{
        architecture::AdrPhase,
        discovery::DiscoveryPhase,
        work_item::{WorkItem, WorkItemStatus},
    },
    types::{
        design::AtomicKind,
        gate::{GateKind, GateVerdict, VetoReason},
        ids::{AdrId, ComponentId, WorkItemId},
        metrics::StepOutcome,
        phase::PhaseKind,
        routing::WorkType,
        tdd::TddPhase,
    },
};
use cfk_engine::{
    architecture::project_architecture_md,
    checks::load_checks,
    commands::{CommandError, handle_claim, handle_metrics, handle_next_step, handle_record_outcome, validate_gate_verdict},
    config::default_routing_table,
    emc::read_verified_slices,
    events::{FactoryEvent, append_event},
    forge::{ForgeAdapter, GiteaForge, MemoryForge},
    loader::{apply_event, load_project_state},
    project::ProjectState,
    review::{handle_pr_merge, handle_pr_poll},
    runner::run_check,
};
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
    /// Restrict to a specific phase. Accepted values: `discovery`,
    /// `event_modeling`, `architecture`, `design_system`, `development`,
    /// `review`.
    pub phase: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NextStepParams {
    /// Optional phase filter.
    pub phase: Option<String>,
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
    pub phase: String,
    /// Work type.
    pub work_type: String,
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
pub struct DesignAddComponentParams {
    /// The `work_item_id` of the design-system work item.
    pub work_item_id: String,
    /// Component name (e.g. `"PrimaryButton"`).
    pub name: String,
    /// Atomic Design level: `quark` | `atom` | `molecule` | `organism` | `template` | `page`.
    pub kind: String,
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
    phase: String,
    ready: usize,
    in_progress: usize,
    blocked: usize,
    done: usize,
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
    event_sequence: u64,
}

impl ServerState {
    fn new(project_root: PathBuf, project: Option<ProjectState>, event_sequence: u64) -> Self {
        Self { project_root, project, event_sequence }
    }

    fn next_seq(&mut self) -> u64 {
        self.event_sequence += 1;
        self.event_sequence
    }

    fn emit(&mut self, event: FactoryEvent) -> anyhow::Result<()> {
        let seq = self.next_seq();
        let envelope = append_event(&self.project_root, seq, event)?;
        if let Some(ref mut proj) = self.project {
            apply_event(proj, &envelope.payload);
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
    // Used by the `#[tool_handler]` macro; dead-code lint doesn't see the use.
    #[expect(dead_code, reason = "field is read by the #[tool_handler] macro expansion; rustc's dead-code analysis does not see macro-generated references")]
    tool_router: ToolRouter<Self>,
}

impl CfkServer {
    /// Load existing project state (if any) and return a ready server.
    ///
    /// Uses `GiteaForge` when `GITEA_URL`/`GITEA_TOKEN`/`GITEA_OWNER`/`GITEA_REPO`
    /// are set; otherwise falls back to an in-memory forge (for local dev/testing).
    ///
    /// # Errors
    /// Returns an error if the event log cannot be read.
    pub fn load(project_root: PathBuf) -> anyhow::Result<Self> {
        let forge: Arc<dyn ForgeAdapter> = match GiteaForge::from_env() {
            Ok(f) => f,
            Err(_) => MemoryForge::new(),
        };
        Self::load_with_forge(project_root, forge)
    }

    /// Load with an explicit forge adapter (used in tests).
    ///
    /// # Errors
    /// Returns an error if the event log cannot be read.
    pub fn load_with_forge(
        project_root: PathBuf,
        forge: Arc<dyn ForgeAdapter>,
    ) -> anyhow::Result<Self> {
        let project = load_project_state(&project_root)?;
        let event_sequence: u64 = {
            let dir = cfk_engine::store::event_export_dir(&project_root);
            if dir.exists() {
                std::fs::read_dir(&dir)
                    .ok()
                    .map_or(0, std::iter::Iterator::count) as u64
            } else {
                0
            }
        };

        Ok(Self {
            state: Arc::new(RwLock::new(ServerState::new(
                project_root,
                project,
                event_sequence,
            ))),
            forge,
            tool_router: Self::tool_router(),
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn tool_error(msg: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![Content::text(msg)])
}

fn parse_phase(s: &str) -> Option<PhaseKind> {
    match s {
        "discovery" => Some(PhaseKind::Discovery),
        "event_modeling" => Some(PhaseKind::EventModeling),
        "architecture" => Some(PhaseKind::Architecture),
        "design_system" => Some(PhaseKind::DesignSystem),
        "development" => Some(PhaseKind::Development),
        "review" => Some(PhaseKind::Review),
        _ => None,
    }
}

fn parse_work_type(s: &str) -> Option<WorkType> {
    match s {
        "socratic_discovery" => Some(WorkType::SocraticDiscovery),
        "event_model_authoring" => Some(WorkType::EventModelAuthoring),
        "adr_drafting" => Some(WorkType::AdrDrafting),
        "adr_review" => Some(WorkType::AdrReview),
        "design_system_build" => Some(WorkType::DesignSystemBuild),
        "outer_behavioral_test_writing" => Some(WorkType::OuterBehavioralTestWriting),
        "test_review" => Some(WorkType::TestReview),
        "narrowest_step_implementation" => Some(WorkType::NarrowestStepImplementation),
        "implementation_review" => Some(WorkType::ImplementationReview),
        "mechanical_transform" => Some(WorkType::MechanicalTransform),
        "pr_comment_triage" => Some(WorkType::PrCommentTriage),
        "research" => Some(WorkType::Research),
        _ => None,
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

fn content_json<T: Serialize>(value: &T) -> Result<CallToolResult, McpError> {
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
    async fn cf_init(
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

        let seq = guard.next_seq();
        let envelope = append_event(
            &root,
            seq,
            FactoryEvent::ProjectInitialized { id: project_id.clone() },
        )
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        guard.project = Some(project_state);
        apply_event(
            guard.project.as_mut().expect("just set above"),
            &envelope.payload,
        );

        content_json(&serde_json::json!({
            "project_id": project_id.to_string(),
            "root": root.display().to_string(),
        }))
    }

    #[tool(description = "\
Return a compact dashboard of work-item counts per phase. \
Shows ready / in_progress / blocked / done for each phase.")]
    async fn cf_status(
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
                    phase: format!("{phase:?}").to_lowercase(),
                    ready: c.ready,
                    in_progress: c.in_progress,
                    blocked: c.blocked,
                    done: c.done,
                }
            })
            .filter(|p| p.ready > 0 || p.in_progress > 0 || p.blocked > 0 || p.done > 0)
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
    async fn cf_next_step(
        &self,
        Parameters(params): Parameters<NextStepParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut guard = self.state.write().await;
        let Some(ref proj) = guard.project else {
            return Ok(tool_error("Project not initialized. Run `cf_init` first."));
        };

        let phase_filter = params.phase.as_deref().and_then(parse_phase);

        // Auto-claim ready development items if session_identity is provided.
        if let Some(ref session_id) = params.session_identity {
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
                    .map_err(|e: CommandError| McpError::invalid_params(e.to_string(), None))?;

                guard
                    .emit(FactoryEvent::LeaseGranted { lease })
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                // Start TDD slice state.
                guard
                    .emit(FactoryEvent::TddSliceStarted {
                        work_item_id: wid,
                        author_identity: session_id.clone(),
                    })
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            }
        }

        let proj = guard.project.as_ref().expect("checked above");
        let response = handle_next_step(proj, phase_filter)
            .map_err(|e: CommandError| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::to_value(&response).map_err(|e| {
            McpError::internal_error(format!("serialization error: {e}"), None)
        })?)
    }

    #[tool(description = "\
Claim a work item for this session. Returns a `lease_id` and `granted_at` \
timestamp. The conductor must hold the lease while executing the step and \
release it on completion or failure.")]
    async fn cf_claim(
        &self,
        Parameters(params): Parameters<ClaimParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut guard = self.state.write().await;
        let Some(ref proj) = guard.project else {
            return Ok(tool_error("Project not initialized."));
        };

        let work_item_id = parse_work_item_id(&params.work_item_id)?;

        let lease = handle_claim(proj, &work_item_id, &params.session_identity)
            .map_err(|e: CommandError| McpError::invalid_params(e.to_string(), None))?;

        let lease_id_str = lease.id.to_string();
        let granted_at = lease.granted_at;

        guard
            .emit(FactoryEvent::LeaseGranted { lease })
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
    async fn cf_release(
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::json!({ "released": params.work_item_id }))
    }

    #[tool(description = "\
Submit the result of a step, advancing the TDD state machine.\n\n\
For `WriteTest` phase: `result` must be `{\"test_content\": \"<code>\"}`.\n\
For `Implement` phase: `result` may include `{\"drill_down_description\": \"<desc>\"}`\n\
if the error requires a tighter unit test; omit or set to null otherwise.\n\
For other phases: any JSON value is accepted as evidence.")]
    async fn cf_submit(
        &self,
        Parameters(params): Parameters<SubmitParams>,
    ) -> Result<CallToolResult, McpError> {
        let work_item_id = parse_work_item_id(&params.work_item_id)?;

        // Extract state into owned values to release the immutable borrow before emit.
        let (current_tdd_phase, item_status, item_work_type, author_identity, frame_depth,
             triage_review_info) = {
            let guard = self.state.read().await;
            let proj = guard.project.as_ref()
                .ok_or_else(|| McpError::invalid_params("Project not initialized.", None))?;

            if !proj.work_items.iter().any(|i| i.id == work_item_id) {
                return Ok(tool_error(format!("work item {} not found", params.work_item_id)));
            }

            let item = proj.work_items.iter().find(|i| i.id == work_item_id);
            let status = item.map(|i| i.status);
            let work_type = item.map(|i| i.work_type);

            let phase = proj.dev_states.get(&work_item_id)
                .and_then(|d| d.current_phase()).cloned();

            let author = proj.leases.iter()
                .find(|l| l.work_item_id == work_item_id)
                .map_or_else(|| "unknown".to_string(), |l| l.session_identity.to_string());

            let depth = proj.dev_states.get(&work_item_id)
                .and_then(|d| d.current_frame()).map_or(0, |f| f.depth);

            // If this is a triage item, find its parent review work item + comment_id.
            let triage_info = proj.review_states.iter().find_map(|(review_wid, rs)| {
                rs.pending_triage.iter().find(|(_, tid)| tid == &work_item_id)
                    .map(|(cid, _)| {
                        let pr_number = rs.pr_number.unwrap_or(0);
                        (review_wid.clone(), cid.clone(), pr_number)
                    })
            });

            (phase, status, work_type, author, depth, triage_info)
        };

        let mut guard = self.state.write().await;

        match current_tdd_phase {
            Some(TddPhase::WriteTest) => {
                let test_content = params
                    .result.get("test_content")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| McpError::invalid_params(
                        "WriteTest submission must include {\"test_content\": \"...\"}",
                        None,
                    ))?.to_string();

                guard.emit(FactoryEvent::TddTestSubmitted {
                    work_item_id,
                    frame_depth,
                    test_content: test_content.clone(),
                    author_identity,
                }).map_err(|e| McpError::internal_error(e.to_string(), None))?;

                content_json(&serde_json::json!({
                    "work_item_id": params.work_item_id,
                    "advanced_to": "test_review_gate",
                    "test_content_length": test_content.len(),
                }))
            }

            Some(TddPhase::Implement) => {
                let drill_down = params.result.get("drill_down_description")
                    .and_then(serde_json::Value::as_str).map(String::from);

                if let Some(desc) = drill_down {
                    guard.emit(FactoryEvent::TddDrillDownPushed {
                        work_item_id,
                        child_description: desc.clone(),
                        child_depth: frame_depth + 1,
                    }).map_err(|e| McpError::internal_error(e.to_string(), None))?;

                    return content_json(&serde_json::json!({
                        "work_item_id": params.work_item_id,
                        "advanced_to": "write_test",
                        "drill_down_depth": frame_depth + 1,
                        "description": desc,
                    }));
                }

                guard.emit(FactoryEvent::TddPhaseAdvanced {
                    work_item_id,
                    frame_depth,
                    new_phase: TddPhase::CheckProgress,
                }).map_err(|e| McpError::internal_error(e.to_string(), None))?;

                content_json(&serde_json::json!({
                    "work_item_id": params.work_item_id,
                    "advanced_to": "check_progress",
                }))
            }

            Some(other) => {
                content_json(&serde_json::json!({
                    "work_item_id": params.work_item_id,
                    "current_tdd_phase": tdd_phase_label(&other),
                    "note": "Submission acknowledged; use cf_gate for gate verdicts.",
                }))
            }

            None => {
                if item_status != Some(WorkItemStatus::InProgress) {
                    return Ok(tool_error(format!(
                        "work item {} is not in progress",
                        params.work_item_id
                    )));
                }

                // If this is a PrCommentTriage item, post the comment to the forge
                // before completing the work item.
                if item_work_type == Some(WorkType::PrCommentTriage)
                    && let Some((review_wid, comment_id, pr_number)) = triage_review_info
                {
                    let reply_body = params.result
                        .get("reply")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_else(|| {
                            params.result.as_str().unwrap_or("(no reply provided)")
                        })
                        .to_string();

                    self.forge
                        .post_comment(pr_number, &reply_body)
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                    guard.emit(FactoryEvent::ReviewCommentPosted {
                        review_work_item_id: review_wid,
                        comment_id,
                        triage_item_id: work_item_id.clone(),
                    }).map_err(|e| McpError::internal_error(e.to_string(), None))?;
                }

                guard.emit(FactoryEvent::WorkItemCompleted { work_item_id })
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                content_json(&serde_json::json!({ "completed": params.work_item_id }))
            }
        }
    }

    #[tool(description = "\
Record a gate verdict (test review, implementation review, or ADR review). \
The reviewer identity must differ from the work item's claiming session. \
For TDD gates: when vetoed, the cycle loops back; when approved, it advances. \
For ADR review: `approved` accepts the ADR into the registry; `vetoed` rejects it. \
`reason` is required when vetoed.")]
    async fn cf_gate(
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

        reviewer_ok.map_err(|e: CommandError| McpError::invalid_params(e.to_string(), None))?;

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
                .expect("adr gate always has adr_info");
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
            }).map_err(|e| McpError::internal_error(e.to_string(), None))?;

            guard.emit(FactoryEvent::WorkItemCompleted {
                work_item_id: work_item_id.clone(),
            }).map_err(|e| McpError::internal_error(e.to_string(), None))?;

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
            // AdrReview handled above; unreachable here.
            (GateKind::AdrReview, _) => unreachable!("ADR gates return early"),
        };

        let new_phase_label = tdd_phase_label(&new_phase);

        let mut guard = self.state.write().await;
        guard
            .emit(FactoryEvent::TddGateVerdict {
                work_item_id: work_item_id.clone(),
                gate_kind,
                verdict,
                reviewer_id: params.reviewer_id.clone(),
            })
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::json!({
            "work_item_id": params.work_item_id,
            "reviewer_id": params.reviewer_id,
            "advanced_to": new_phase_label,
        }))
    }

    #[tool(description = "\
List all work items in the backlog, optionally filtered by phase.")]
    async fn cf_backlog(
        &self,
        Parameters(params): Parameters<PhaseFilterParams>,
    ) -> Result<CallToolResult, McpError> {
        let guard = self.state.read().await;
        let Some(ref proj) = guard.project else {
            return Ok(tool_error("Project not initialized."));
        };

        let phase_filter = params.phase.as_deref().and_then(parse_phase);

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
    async fn cf_backlog_add(
        &self,
        Parameters(params): Parameters<BacklogAddParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut guard = self.state.write().await;
        if guard.project.is_none() {
            return Ok(tool_error("Project not initialized."));
        }

        let phase = parse_phase(&params.phase).ok_or_else(|| {
            McpError::invalid_params(format!("unknown phase: {}", params.phase), None)
        })?;

        let work_type = parse_work_type(&params.work_type).ok_or_else(|| {
            McpError::invalid_params(format!("unknown work_type: {}", params.work_type), None)
        })?;

        let item = WorkItem::new(
            cfk_core::types::ids::WorkItemId::new(),
            phase,
            work_type,
            params.description,
        );

        let item_id = item.id.to_string();

        guard
            .emit(FactoryEvent::WorkItemAdded { work_item: item })
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::json!({ "work_item_id": item_id }))
    }

    #[tool(description = "Show the current routing table (work type → executor mapping).")]
    async fn cf_route(
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
Ingest slices from a formally-verified emc model into the development backlog. \
Reads `model/events/v1/` under the product repo, finds all `SliceAdded` events \
whose workflow has a `WorkflowReadinessDeclared` event, and creates a development \
work item for each new slice (idempotent — already-ingested slugs are skipped). \
Returns counts of ingested and skipped slices.")]
    async fn cf_ingest_slices(
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

        // Collect already-ingested slugs to avoid duplicates.
        let existing_slugs: std::collections::HashSet<String> = proj
            .work_items
            .iter()
            .filter_map(|i| i.emc_slug.clone())
            .collect();

        let mut ingested = 0usize;
        let mut skipped = 0usize;
        let mut new_ids: Vec<String> = Vec::new();

        for slice in slices {
            if existing_slugs.contains(&slice.slug) {
                skipped += 1;
                continue;
            }

            let work_type = match slice.kind.as_str() {
                "translation" | "mechanical" => WorkType::MechanicalTransform,
                _ => WorkType::NarrowestStepImplementation,
            };

            let item = WorkItem::from_emc_slice(
                cfk_core::types::ids::WorkItemId::new(),
                PhaseKind::Development,
                work_type,
                format!("[{}] {}", slice.slug, slice.name),
                slice.slug.clone(),
            );

            let id_str = item.id.to_string();

            guard
                .emit(FactoryEvent::WorkItemAdded { work_item: item })
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            new_ids.push(id_str);
            ingested += 1;
        }

        content_json(&serde_json::json!({
            "ingested": ingested,
            "skipped": skipped,
            "work_item_ids": new_ids,
        }))
    }

    #[tool(description = "\
Run a configured deterministic check (tests, linter, build) and record the \
result as evidence. Agents never self-report pass/fail — the kernel always \
runs checks itself.\n\n\
Provide `work_item_id` to advance the TDD state machine based on the result.")]
    async fn cf_run_check(
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

            let mut guard = self.state.write().await;
            guard
                .emit(FactoryEvent::TddCheckResult {
                    work_item_id,
                    check_name: params.check_name.clone(),
                    passed: result.passed,
                    first_error: result.first_error.clone(),
                })
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
        if let Some(phase) = tdd_phase {
            resp["advanced_tdd_phase_to"] = serde_json::Value::String(phase.to_string());
        }

        content_json(&resp)
    }

    #[tool(description = "\
Open a pull request on the forge for a review-phase slice. \
Returns the PR number and URL. `head` is the source branch; `base` is the \
target branch (usually `main`).")]
    async fn cf_pr_open(
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

        let pr_number = opened.number;
        let pr_url = opened.url.clone();

        let mut guard = self.state.write().await;
        guard.emit(FactoryEvent::ReviewSliceStarted {
            work_item_id,
            pr_number,
            pr_url: pr_url.clone(),
        }).map_err(|e| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::json!({
            "work_item_id": params.work_item_id,
            "pr_number": pr_number,
            "pr_url": pr_url,
        }))
    }

    #[tool(description = "\
Poll the forge for a review-phase PR: CI status, review approvals, and new \
comments. New comments produce `PrCommentTriage` work items. \
Returns a summary of CI status and any new triage items created.")]
    async fn cf_pr_poll(
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
                    let triage_item = cfk_core::state_machine::work_item::WorkItem::new(
                        triage_item_id.clone(),
                        cfk_core::types::phase::PhaseKind::Review,
                        WorkType::PrCommentTriage,
                        format!("Comment {comment_id}: {}", &comment_body[..comment_body.len().min(120)]),
                    );
                    triage_ids.push(triage_item_id.to_string());
                    let mut guard = self.state.write().await;
                    guard.emit(FactoryEvent::WorkItemAdded { work_item: triage_item })
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    guard.emit(event.clone())
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                }
                FactoryEvent::ReviewAllGreen { .. } => {
                    all_green = true;
                    let mut guard = self.state.write().await;
                    guard.emit(event.clone())
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
    async fn cf_pr_merge(
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
            guard.emit(event).map_err(|e| McpError::internal_error(e.to_string(), None))?;
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
    async fn cf_discovery_submit(
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
        }).map_err(|e| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::json!({
            "work_item_id": params.work_item_id,
            "workflows": params.workflows,
        }))
    }

    #[tool(description = "\
Approve or reject a discovery brief (human gate). If approved, all submitted \
workflows are queued as event-modeling work items and the discovery work item \
is completed. If rejected, the discovery dialogue resets for a re-run.")]
    async fn cf_discovery_approve(
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
        }).map_err(|e| McpError::internal_error(e.to_string(), None))?;

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
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            }

            guard.emit(FactoryEvent::WorkItemCompleted {
                work_item_id: work_item_id.clone(),
            }).map_err(|e| McpError::internal_error(e.to_string(), None))?;
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
    async fn cf_adr_submit(
        &self,
        Parameters(params): Parameters<AdrSubmitParams>,
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
        }).map_err(|e| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::json!({
            "work_item_id": params.work_item_id,
            "adr_id": adr_id.to_string(),
            "title": params.title,
        }))
    }

    // ── Design-system phase tools ─────────────────────────────────────────

    #[tool(description = "\
Add a design component to the Atomic Design inventory (called by the \
design-system agent). Marks the work item as done. \
`kind`: `quark` | `atom` | `molecule` | `organism` | `template` | `page`.")]
    async fn cf_design_add_component(
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

        let component_id = ComponentId::new();

        let mut guard = self.state.write().await;
        guard.emit(FactoryEvent::DesignComponentAdded {
            work_item_id: work_item_id.clone(),
            component_id: component_id.clone(),
            name: params.name.clone(),
            kind,
            slice_ref: params.slice_ref.clone(),
        }).map_err(|e| McpError::internal_error(e.to_string(), None))?;

        guard.emit(FactoryEvent::WorkItemCompleted {
            work_item_id: work_item_id.clone(),
        }).map_err(|e| McpError::internal_error(e.to_string(), None))?;

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
    async fn cf_design_cross_check(
        &self,
        Parameters(params): Parameters<DesignCrossCheckParams>,
    ) -> Result<CallToolResult, McpError> {
        let existing_names: std::collections::HashSet<String> = {
            let guard = self.state.read().await;
            let proj = guard.project.as_ref()
                .ok_or_else(|| McpError::invalid_params("Project not initialized.", None))?;
            proj.design_inventory.iter().map(|c| c.name.clone()).collect()
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
            }).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        content_json(&serde_json::json!({
            "new_work_item_ids": new_item_ids,
            "gap_count": new_item_ids.len(),
        }))
    }

    #[tool(description = "\
Return aggregated per-work-type metrics: veto rates and average token costs. \
Use this to justify routing table defaults and identify work types where the \
current executor is under-performing (high veto rate = consider a stronger \
model or a different provider). Returns entries sorted by veto rate descending.")]
    async fn cf_metrics(
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
    async fn cf_record_outcome(
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
