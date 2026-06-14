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
    derive(Debug, Display, Clone, PartialEq, Eq, Serialize, Deserialize)
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

impl HumanQuestion {
    /// Construct a `HumanQuestion` from a `&'static str` literal.
    ///
    /// Callers must only pass non-empty string literals.
    ///
    /// # Panics
    ///
    /// Panics if `s` is empty or whitespace-only. Because the input must be a
    /// `&'static str` literal this is a compile-time authoring error, not a
    /// runtime condition.
    #[must_use]
    pub fn from_static(s: &'static str) -> Self {
        #[expect(
            clippy::expect_used,
            reason = "called only with non-empty &'static str literals; emptiness is a compile-time authoring error, not a runtime condition"
        )]
        Self::try_new(s.to_string()).expect("static HumanQuestion literal must be non-empty")
    }
}

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

/// Well-known `CheckName` constants for the built-in test and lint checks.
///
/// Using these constants instead of `CheckName::try_new("tests")` avoids scattered
/// `expect` calls for literals that are statically valid.
pub mod well_known {
    use super::CheckName;
    use std::sync::LazyLock;

    fn known(s: &'static str) -> CheckName {
        #[expect(
            clippy::expect_used,
            reason = "called only with non-empty &'static str literals; a unit test below proves every literal validates, making this path unreachable in practice"
        )]
        CheckName::try_new(s.to_string()).expect("static check name is non-empty")
    }

    /// The standard test-suite check (`cargo nextest run` by default).
    pub static TESTS: LazyLock<CheckName> = LazyLock::new(|| known("tests"));

    /// The standard linter check (`cargo clippy -- -D warnings` by default).
    pub static LINT: LazyLock<CheckName> = LazyLock::new(|| known("lint"));

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn well_known_check_names_are_valid() {
            let _ = &*TESTS;
            let _ = &*LINT;
        }
    }
}
