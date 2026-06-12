//! TDD slice state — tracks each development work item through the full
//! red-green-refactor cycle including drill-down frame stacks.

use crate::types::ids::WorkItemId;
use nutype::nutype;
use serde::{Deserialize, Serialize};

/// Test code submitted by the test-writer agent.
#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(Debug, Display, Clone, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct TestCode(String);

/// A compiler or test error message the implementer must address.
#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(Debug, Display, Clone, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct ErrorMessage(String);

/// Session identity of whoever claimed a work item (the "author").
/// Gate reviewers must have a different identity.
#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(Debug, Display, Clone, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct AuthorIdentity(String);

/// Session identity of a gate reviewer.
#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(Debug, Display, Clone, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct ReviewerId(String);

/// A description of the tighter unit test needed for a drill-down.
#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct DrillDownDescription(String);

/// The current phase of one TDD frame (one level of the drill-down stack).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TddPhase {
    /// Waiting for the test-writer agent to submit test code.
    WriteTest,
    /// Test submitted; waiting for independent test-reviewer gate.
    TestReviewGate,
    /// Test approved; kernel must run the suite and confirm expected failure.
    RedCheck,
    /// Red confirmed; waiting for implementer agent.
    Implement,
    /// Implementation submitted; kernel must run the suite to check progress.
    CheckProgress,
    /// Implementation green; waiting for independent implementation-reviewer gate.
    ImplReviewGate,
    /// Implementation approved; kernel must run the linter.
    LintCheck,
    /// Lint passed; this frame is complete (pop if nested, commit if outermost).
    Done,
}

/// One frame on the TDD drill-down stack.
///
/// A new frame is pushed whenever the implementer signals that the current
/// error requires changes across more than one function (drill-down into a
/// tighter unit test).  The outermost frame (depth 0) represents the outer
/// behavioural test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TddFrame {
    /// 0 = outer test; ≥1 = drill-down nesting level.
    pub depth: u32,
    pub phase: TddPhase,
    /// The test code submitted by the test-writer (set after `WriteTest`).
    pub test_content: Option<TestCode>,
    /// The reason the kernel confirmed the test fails (set after `RedCheck`).
    pub expected_failure: Option<ErrorMessage>,
    /// The first compiler/test error the implementer must address next.
    pub current_error: Option<ErrorMessage>,
    /// Session identity of whoever claimed this work item (the "author").
    /// Gate reviewers must have a different identity.
    pub author_identity: Option<AuthorIdentity>,
}

impl TddFrame {
    #[must_use]
    pub fn new(depth: u32) -> Self {
        Self {
            depth,
            phase: TddPhase::WriteTest,
            test_content: None,
            expected_failure: None,
            current_error: None,
            author_identity: None,
        }
    }
}

/// Full TDD state for one development work item.
///
/// `frames` is the drill-down stack; the last element is the active frame.
/// When a nested frame completes (reaches `Done`), it is popped and the
/// parent frame resumes its `Implement` loop with the same `current_error`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevSliceState {
    pub work_item_id: WorkItemId,
    /// Stack of TDD frames.  Never empty while the slice is in progress.
    pub frames: Vec<TddFrame>,
}

impl DevSliceState {
    /// Create a fresh `DevSliceState` with an initial outer test frame.
    #[must_use]
    pub fn new(work_item_id: WorkItemId) -> Self {
        Self {
            work_item_id,
            frames: vec![TddFrame::new(0)],
        }
    }

    #[must_use]
    pub fn current_frame(&self) -> Option<&TddFrame> {
        self.frames.last()
    }

    #[must_use]
    pub fn current_frame_mut(&mut self) -> Option<&mut TddFrame> {
        self.frames.last_mut()
    }

    #[must_use]
    pub fn current_phase(&self) -> Option<&TddPhase> {
        self.frames.last().map(|f| &f.phase)
    }
}
