//! Routing table types — maps work types to executor specifications.

use nutype::nutype;
use serde::{Deserialize, Serialize};

/// The category of work, used to look up the executor in the routing table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkType {
    SocraticDiscovery,
    EventModelAuthoring,
    AdrDrafting,
    AdrReview,
    DesignSystemBuild,
    OuterBehavioralTestWriting,
    TestReview,
    NarrowestStepImplementation,
    ImplementationReview,
    MechanicalTransform,
    PrCommentTriage,
    Research,
}

/// Which LLM provider to use for this work type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    /// Invoke via the Claude Code Agent tool.
    Claude,
    /// Invoke via `scripts/codex-runner.sh` (GPT, subscription billing).
    Codex,
}

/// Model tier within the Claude provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClaudeModel {
    Haiku,
    Sonnet,
    Opus,
    Inherit,
}

/// Model identifier for Codex/GPT executors.
#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct CodexModel(String);

/// Effort level for Codex executors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CodexEffort {
    Low,
    Medium,
    High,
}

/// The resolved executor for a work type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum ExecutorSpec {
    Claude {
        model: ClaudeModel,
        agent_name: AgentName,
    },
    Codex {
        model: CodexModel,
        effort: CodexEffort,
    },
}

/// The name of a Claude Code agent defined in the plugin's agents/ directory.
#[nutype(
    sanitize(trim, lowercase),
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct AgentName(String);

/// A single entry in the routing table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingEntry {
    pub work_type: WorkType,
    pub executor: ExecutorSpec,
    pub notes: Option<String>,
}

/// The full routing table, loaded from TOML config.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingTable {
    pub entries: Vec<RoutingEntry>,
}

impl RoutingTable {
    /// Look up the executor for a work type. Returns `None` if no entry exists.
    #[must_use]
    pub fn resolve(&self, work_type: WorkType) -> Option<&ExecutorSpec> {
        self.entries
            .iter()
            .find(|e| e.work_type == work_type)
            .map(|e| &e.executor)
    }
}
