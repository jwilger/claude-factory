//! MCP server implementation — exposes `cf_*` tools over stdio.

use cfk_core::{
    state_machine::work_item::{WorkItem, WorkItemStatus},
    types::{
        ids::WorkItemId,
        phase::PhaseKind,
        routing::WorkType,
    },
};
use cfk_engine::{
    commands::{CommandError, handle_claim, handle_next_step},
    config::default_routing_table,
    events::{FactoryEvent, append_event},
    loader::{apply_event, load_project_state},
    project::ProjectState,
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
//
// Use primitive Rust types only — nutype newtypes don't implement `JsonSchema`.
// IDs arrive as strings and are parsed inside each handler.

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
    /// The `work_item_id` returned by the original `cf_next_step` call.
    pub work_item_id: String,
    /// Free-form result text or JSON from the executor.
    pub result: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BacklogAddParams {
    /// Phase for this work item: `discovery`, `event_modeling`, `architecture`,
    /// `design_system`, `development`, or `review`.
    pub phase: String,
    /// Work type: `socratic_discovery`, `event_model_authoring`, `adr_drafting`,
    /// `outer_behavioral_test_writing`, `test_review`,
    /// `narrowest_step_implementation`, `implementation_review`,
    /// `mechanical_transform`, `pr_comment_triage`, or `research`.
    pub work_type: String,
    /// Human-readable description of the work.
    pub description: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RunCheckParams {
    /// Name of the configured check to run.
    pub check_name: String,
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
        Self {
            project_root,
            project,
            event_sequence,
        }
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
    // Used by the `#[tool_handler]` macro; dead-code lint doesn't see the use.
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl CfkServer {
    /// Load existing project state (if any) and return a ready server.
    ///
    /// # Errors
    /// Returns an error if the event log cannot be read.
    pub fn load(project_root: PathBuf) -> anyhow::Result<Self> {
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
            FactoryEvent::ProjectInitialized {
                id: project_id.clone(),
            },
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
Return the next work step for the conductor to execute. \
The response includes `step_id`, `work_item_id`, `action` \
(`spawn_agent` / `run_check` / `ask_human` / `idle`), `executor` spec, \
and `prompt`. When `action` is `idle`, no work is currently ready.")]
    async fn cf_next_step(
        &self,
        Parameters(params): Parameters<PhaseFilterParams>,
    ) -> Result<CallToolResult, McpError> {
        let guard = self.state.read().await;
        let Some(ref proj) = guard.project else {
            return Ok(tool_error("Project not initialized. Run `cf_init` first."));
        };

        let phase_filter = params.phase.as_deref().and_then(parse_phase);

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
            .emit(FactoryEvent::LeaseReleased {
                lease_id,
                work_item_id,
            })
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::json!({ "released": params.work_item_id }))
    }

    #[tool(description = "\
Submit the result of a step. In M1 this marks the work item as `Done`. \
Full gate validation (test review, implementation review, lint) is enforced \
from M2 onward.")]
    async fn cf_submit(
        &self,
        Parameters(params): Parameters<SubmitParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut guard = self.state.write().await;
        let Some(ref proj) = guard.project else {
            return Ok(tool_error("Project not initialized."));
        };

        let work_item_id = parse_work_item_id(&params.work_item_id)?;

        let found = proj
            .work_items
            .iter()
            .any(|i| i.id == work_item_id && i.status == WorkItemStatus::InProgress);

        if !found {
            return Ok(tool_error(format!(
                "work item {} is not in progress",
                params.work_item_id
            )));
        }

        guard
            .emit(FactoryEvent::WorkItemCompleted {
                work_item_id: work_item_id.clone(),
            })
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        content_json(&serde_json::json!({
            "completed": params.work_item_id,
            "result_length": params.result.len(),
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
Run a configured deterministic check (tests, linter, build) and record the \
result as evidence. Agents never self-report pass/fail — the kernel always \
runs checks itself.\n\n\
**M1 stub**: returns a placeholder. Full implementation in M2.")]
    async fn cf_run_check(
        &self,
        Parameters(params): Parameters<RunCheckParams>,
    ) -> Result<CallToolResult, McpError> {
        content_json(&serde_json::json!({
            "check_name": params.check_name,
            "status": "stub",
            "message": "cf_run_check is not yet implemented (M1 stub). Full implementation in M2.",
        }))
    }
}

#[tool_handler]
impl ServerHandler for CfkServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}
