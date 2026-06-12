//! Step types — the unit of work the kernel dispatches to the conductor.

use crate::types::{
    gate::GateKind,
    ids::{StepId, WorkItemId},
    phase::PhaseKind,
    routing::ExecutorSpec,
};
use nutype::nutype;
use serde::{Deserialize, Serialize};

/// A human-readable prompt the conductor passes verbatim to the executor.
#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct StepPrompt(String);

/// The action the conductor must take for this step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StepAction {
    /// Spawn an LLM agent with the given executor spec and prompt.
    SpawnAgent {
        executor: ExecutorSpec,
        prompt: StepPrompt,
        /// JSON Schema the agent's output must conform to (if present).
        output_schema: Option<serde_json::Value>,
    },
    /// The kernel will run a deterministic check itself (tests, linter, etc.).
    RunCheck {
        check_name: CheckName,
    },
    /// Spawn a reviewer agent; conductor must call `cf_gate` with the verdict.
    /// The reviewer identity is enforced to differ from the work item author.
    GateReview {
        gate_kind: GateKind,
        executor: ExecutorSpec,
        prompt: StepPrompt,
    },
    /// A human decision is needed before the step can proceed.
    AskHuman {
        question: HumanQuestion,
    },
    /// Conductor must open a PR with the given prompt (title/body details inside).
    OpenPr {
        prompt: StepPrompt,
    },
    /// Conductor must call `cf_pr_poll` to check CI, reviews, and new comments.
    RunPrPoll,
    /// PR is all-green and approved; conductor must call `cf_pr_merge`.
    MergePr,
}

/// The name of a deterministic check configured in the project.
#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct CheckName(String);

/// A question presented to the human operator for a required decision.
#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct HumanQuestion(String);

/// The reason the factory is idle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdleReason {
    NoReadyWork,
    WaitingOnEscalation,
    AllPhasesComplete,
}

/// A step ready for the conductor to execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyStep {
    pub step_id: StepId,
    pub work_item_id: WorkItemId,
    pub phase: PhaseKind,
    pub action: StepAction,
}
