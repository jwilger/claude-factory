//! Pure TDD slice state machine.
//!
//! All functions are pure: given state + input → (new state, effects) | error.
//! No I/O of any kind. The imperative shell (cfk-engine) drives this by applying
//! inputs and emitting the resulting events.

use crate::types::{
    gate::{GateKind, GateVerdict},
    tdd::{DevSliceState, TddFrame, TddPhase},
};
use thiserror::Error;

/// Inputs that drive TDD state transitions.
#[derive(Debug, Clone)]
pub enum TddInput {
    /// Test-writer agent submitted test code.
    TestSubmitted { content: String, author_identity: String },
    /// A gate reviewer recorded a verdict.
    GateVerdicted { kind: GateKind, verdict: GateVerdict, reviewer_id: String },
    /// The kernel ran the test suite and has a result.
    CheckResult { passed: bool, first_error: Option<String> },
    /// After impl submission: kernel ran check and compared failure progression.
    ProgressCheckResult {
        passed: bool,
        first_error: Option<String>,
        /// Implementer signalled this error requires a drill-down unit test.
        drill_down_description: Option<String>,
    },
    /// The innermost drill-down frame just went green; pop it and resume.
    DrillDownComplete,
}

/// What changed as a result of a transition (used to emit events in cfk-engine).
#[derive(Debug, Clone)]
pub enum TddEffect {
    PhaseChanged { new_phase: TddPhase },
    TestRecorded { content: String, author: String },
    GateVerdictRecorded { kind: GateKind, verdict: GateVerdict, reviewer: String },
    FailureConfirmed { first_error: String },
    CheckProgressRecorded { passed: bool, first_error: Option<String> },
    DrillDownPushed { description: String, depth: u32 },
    DrillDownPopped,
    SliceDone,
}

/// Errors from TDD state transitions.
#[derive(Debug, Error)]
pub enum TddError {
    #[error("no active TDD frame")]
    NoFrame,

    #[error("input {input:?} is invalid in phase {phase:?}")]
    InvalidInput { input: String, phase: String },

    #[error("reviewer identity '{reviewer}' must differ from author identity '{author}'")]
    ReviewerIsAuthor { reviewer: String, author: String },

    #[error("test passed during RedCheck — test must fail before implementation begins")]
    RedCheckPassed,
}

/// Apply one input to the current TDD state, mutating `state` in place and
/// returning the list of effects to record as events.
///
/// # Errors
/// Returns `TddError` if the input is invalid for the current phase.
///
/// # Panics
/// Panics if the frame stack is non-empty at entry but becomes empty mid-
/// transition — this cannot happen in practice because the match arms that
/// call `expect("frame exists")` are guarded by the initial `NoFrame` check.
#[allow(clippy::too_many_lines)]
pub fn transition(
    state: &mut DevSliceState,
    input: TddInput,
) -> Result<Vec<TddEffect>, TddError> {
    // Clone phase and author to avoid holding a borrow across mutations.
    let (phase, author) = {
        let frame = state.current_frame().ok_or(TddError::NoFrame)?;
        (frame.phase.clone(), frame.author_identity.clone())
    };

    let mut effects = Vec::new();

    match (phase, input) {
        // ── WriteTest ──────────────────────────────────────────────────────
        (TddPhase::WriteTest, TddInput::TestSubmitted { content, author_identity }) => {
            let frame = state.current_frame_mut().expect("frame exists");
            frame.test_content = Some(content.clone());
            frame.author_identity = Some(author_identity.clone());
            frame.phase = TddPhase::TestReviewGate;
            effects.push(TddEffect::TestRecorded { content, author: author_identity });
            effects.push(TddEffect::PhaseChanged { new_phase: TddPhase::TestReviewGate });
        }

        // ── TestReviewGate ────────────────────────────────────────────────
        (TddPhase::TestReviewGate, TddInput::GateVerdicted { kind: GateKind::TestReview, verdict, reviewer_id }) => {
            enforce_reviewer_ne_author(author.as_deref(), &reviewer_id)?;
            let new_phase = if verdict.is_approved() {
                TddPhase::RedCheck
            } else {
                TddPhase::WriteTest
            };
            let frame = state.current_frame_mut().expect("frame exists");
            frame.phase = new_phase.clone();
            effects.push(TddEffect::GateVerdictRecorded { kind: GateKind::TestReview, verdict, reviewer: reviewer_id });
            effects.push(TddEffect::PhaseChanged { new_phase });
        }

        // ── RedCheck ──────────────────────────────────────────────────────
        (TddPhase::RedCheck, TddInput::CheckResult { passed: false, first_error }) => {
            let err = first_error.unwrap_or_else(|| "test failed (no first-error extracted)".to_string());
            let frame = state.current_frame_mut().expect("frame exists");
            frame.expected_failure = Some(err.clone());
            frame.current_error = Some(err.clone());
            frame.phase = TddPhase::Implement;
            effects.push(TddEffect::FailureConfirmed { first_error: err });
            effects.push(TddEffect::PhaseChanged { new_phase: TddPhase::Implement });
        }
        (TddPhase::RedCheck, TddInput::CheckResult { passed: true, .. }) => {
            return Err(TddError::RedCheckPassed);
        }

        // ── Implement → advance to CheckProgress then run progress logic ──
        (TddPhase::Implement, TddInput::CheckResult { passed, first_error }) => {
            // Advance phase, then re-enter as a ProgressCheckResult.
            if let Some(frame) = state.current_frame_mut() {
                frame.phase = TddPhase::CheckProgress;
            }
            let sub_input = TddInput::ProgressCheckResult {
                passed,
                first_error,
                drill_down_description: None,
            };
            return transition(state, sub_input);
        }

        // ── CheckProgress ─────────────────────────────────────────────────
        (TddPhase::CheckProgress, TddInput::ProgressCheckResult { passed: true, .. }) => {
            let frame = state.current_frame_mut().expect("frame exists");
            frame.phase = TddPhase::ImplReviewGate;
            effects.push(TddEffect::CheckProgressRecorded { passed: true, first_error: None });
            effects.push(TddEffect::PhaseChanged { new_phase: TddPhase::ImplReviewGate });
        }
        (TddPhase::CheckProgress, TddInput::ProgressCheckResult {
            passed: false,
            first_error,
            drill_down_description: Some(desc),
        }) => {
            let depth = state.current_frame().expect("frame exists").depth + 1;
            effects.push(TddEffect::CheckProgressRecorded { passed: false, first_error });
            effects.push(TddEffect::DrillDownPushed { description: desc, depth });
            // Parent stays in CheckProgress; push a fresh child frame.
            state.frames.push(TddFrame::new(depth));
        }
        (TddPhase::CheckProgress, TddInput::ProgressCheckResult {
            passed: false,
            first_error,
            drill_down_description: None,
        }) => {
            let err = first_error.unwrap_or_else(|| "still failing".to_string());
            let frame = state.current_frame_mut().expect("frame exists");
            frame.current_error = Some(err.clone());
            frame.phase = TddPhase::Implement;
            effects.push(TddEffect::CheckProgressRecorded { passed: false, first_error: Some(err) });
            effects.push(TddEffect::PhaseChanged { new_phase: TddPhase::Implement });
        }

        // ── DrillDownComplete ─────────────────────────────────────────────
        (_, TddInput::DrillDownComplete) => {
            if state.frames.len() <= 1 {
                return Err(TddError::InvalidInput {
                    input: "DrillDownComplete".to_string(),
                    phase: format!("{:?}", state.current_phase().expect("phase")),
                });
            }
            state.frames.pop();
            effects.push(TddEffect::DrillDownPopped);
            // Parent resumes: advance from CheckProgress back to Implement.
            if let Some(parent) = state.frames.last_mut() {
                parent.phase = TddPhase::Implement;
                effects.push(TddEffect::PhaseChanged { new_phase: TddPhase::Implement });
            }
        }

        // ── ImplReviewGate ────────────────────────────────────────────────
        (TddPhase::ImplReviewGate, TddInput::GateVerdicted { kind: GateKind::ImplementationReview, verdict, reviewer_id }) => {
            enforce_reviewer_ne_author(author.as_deref(), &reviewer_id)?;
            let new_phase = if verdict.is_approved() {
                TddPhase::LintCheck
            } else {
                TddPhase::Implement
            };
            let frame = state.current_frame_mut().expect("frame exists");
            frame.phase = new_phase.clone();
            effects.push(TddEffect::GateVerdictRecorded {
                kind: GateKind::ImplementationReview,
                verdict,
                reviewer: reviewer_id,
            });
            effects.push(TddEffect::PhaseChanged { new_phase });
        }

        // ── LintCheck ─────────────────────────────────────────────────────
        (TddPhase::LintCheck, TddInput::CheckResult { passed: true, .. }) => {
            let frame = state.current_frame_mut().expect("frame exists");
            frame.phase = TddPhase::Done;
            effects.push(TddEffect::SliceDone);
        }
        (TddPhase::LintCheck, TddInput::CheckResult { passed: false, first_error }) => {
            let err = first_error.unwrap_or_else(|| "lint failed".to_string());
            let frame = state.current_frame_mut().expect("frame exists");
            frame.current_error = Some(err);
            frame.phase = TddPhase::Implement;
            effects.push(TddEffect::PhaseChanged { new_phase: TddPhase::Implement });
        }

        // ── Invalid ───────────────────────────────────────────────────────
        (phase, input) => {
            return Err(TddError::InvalidInput {
                input: format!("{input:?}"),
                phase: format!("{phase:?}"),
            });
        }
    }

    Ok(effects)
}

fn enforce_reviewer_ne_author(author: Option<&str>, reviewer: &str) -> Result<(), TddError> {
    if let Some(auth) = author.filter(|a| *a == reviewer) {
        return Err(TddError::ReviewerIsAuthor {
            reviewer: reviewer.to_string(),
            author: auth.to_string(),
        });
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        gate::{GateKind, GateVerdict, VetoReason},
        ids::WorkItemId,
        tdd::DevSliceState,
    };

    fn fresh_state() -> DevSliceState {
        DevSliceState::new(WorkItemId::new())
    }

    fn submit_test(state: &mut DevSliceState) {
        let input = TddInput::TestSubmitted {
            content: "#[test] fn foo() {}".to_string(),
            author_identity: "author-session".to_string(),
        };
        transition(state, input).expect("TestSubmitted");
    }

    fn approve_test_gate(state: &mut DevSliceState) {
        let input = TddInput::GateVerdicted {
            kind: GateKind::TestReview,
            verdict: GateVerdict::Approved,
            reviewer_id: "reviewer-session".to_string(),
        };
        transition(state, input).expect("approve test gate");
    }

    fn confirm_red(state: &mut DevSliceState) {
        let input = TddInput::CheckResult {
            passed: false,
            first_error: Some("error[E0308]: mismatched types".to_string()),
        };
        transition(state, input).expect("RedCheck");
    }

    #[test]
    fn write_test_advances_to_test_review_gate() {
        let mut state = fresh_state();
        submit_test(&mut state);
        assert_eq!(state.current_phase(), Some(&TddPhase::TestReviewGate));
        assert!(state.current_frame().unwrap().test_content.is_some());
    }

    #[test]
    fn approve_gate_advances_to_red_check() {
        let mut state = fresh_state();
        submit_test(&mut state);
        approve_test_gate(&mut state);
        assert_eq!(state.current_phase(), Some(&TddPhase::RedCheck));
    }

    #[test]
    fn veto_gate_returns_to_write_test() {
        let mut state = fresh_state();
        submit_test(&mut state);

        let reason = VetoReason::try_new("Test is implementation-coupled".to_string())
            .expect("valid reason");
        let input = TddInput::GateVerdicted {
            kind: GateKind::TestReview,
            verdict: GateVerdict::Vetoed { reason },
            reviewer_id: "reviewer-session".to_string(),
        };
        transition(&mut state, input).expect("veto gate");
        assert_eq!(state.current_phase(), Some(&TddPhase::WriteTest));
    }

    #[test]
    fn red_check_confirms_failure_and_advances_to_implement() {
        let mut state = fresh_state();
        submit_test(&mut state);
        approve_test_gate(&mut state);
        confirm_red(&mut state);

        assert_eq!(state.current_phase(), Some(&TddPhase::Implement));
        let frame = state.current_frame().unwrap();
        assert!(frame.current_error.is_some());
        assert!(frame.expected_failure.is_some());
    }

    #[test]
    fn red_check_passing_is_an_error() {
        let mut state = fresh_state();
        submit_test(&mut state);
        approve_test_gate(&mut state);

        let input = TddInput::CheckResult { passed: true, first_error: None };
        let err = transition(&mut state, input).expect_err("should fail");
        assert!(matches!(err, TddError::RedCheckPassed));
    }

    #[test]
    fn reviewer_same_as_author_is_rejected() {
        let mut state = fresh_state();
        let input = TddInput::TestSubmitted {
            content: "test code".to_string(),
            author_identity: "same-session".to_string(),
        };
        transition(&mut state, input).expect("submit test");

        let input = TddInput::GateVerdicted {
            kind: GateKind::TestReview,
            verdict: GateVerdict::Approved,
            reviewer_id: "same-session".to_string(),
        };
        let err = transition(&mut state, input).expect_err("same identity should be rejected");
        assert!(matches!(err, TddError::ReviewerIsAuthor { .. }));
    }

    #[test]
    fn green_check_from_implement_advances_to_impl_review() {
        let mut state = fresh_state();
        submit_test(&mut state);
        approve_test_gate(&mut state);
        confirm_red(&mut state);

        let input = TddInput::CheckResult { passed: true, first_error: None };
        transition(&mut state, input).expect("green check");
        assert_eq!(state.current_phase(), Some(&TddPhase::ImplReviewGate));
    }

    #[test]
    fn failing_check_from_implement_loops_back_to_implement() {
        let mut state = fresh_state();
        submit_test(&mut state);
        approve_test_gate(&mut state);
        confirm_red(&mut state);

        let input = TddInput::CheckResult {
            passed: false,
            first_error: Some("error: new different error".to_string()),
        };
        transition(&mut state, input).expect("failing progress check");
        assert_eq!(state.current_phase(), Some(&TddPhase::Implement));
    }

    #[test]
    fn drill_down_pushes_frame_and_pop_resumes_parent() {
        let mut state = fresh_state();
        submit_test(&mut state);
        approve_test_gate(&mut state);
        confirm_red(&mut state);
        // Manually advance to CheckProgress (would normally come via cf_submit).
        state.current_frame_mut().unwrap().phase = TddPhase::CheckProgress;

        let input = TddInput::ProgressCheckResult {
            passed: false,
            first_error: Some("some error".to_string()),
            drill_down_description: Some("unit test for parse_foo".to_string()),
        };
        transition(&mut state, input).expect("drill-down");
        assert_eq!(state.frames.len(), 2, "should have parent + child frame");
        assert_eq!(state.current_phase(), Some(&TddPhase::WriteTest), "child starts in WriteTest");

        transition(&mut state, TddInput::DrillDownComplete).expect("drill-down complete");
        assert_eq!(state.frames.len(), 1, "child popped");
        assert_eq!(state.current_phase(), Some(&TddPhase::Implement), "parent resumes at Implement");
    }

    #[test]
    fn impl_review_approve_advances_to_lint_check() {
        let mut state = fresh_state();
        submit_test(&mut state);
        approve_test_gate(&mut state);
        confirm_red(&mut state);
        state.current_frame_mut().unwrap().phase = TddPhase::ImplReviewGate;

        let input = TddInput::GateVerdicted {
            kind: GateKind::ImplementationReview,
            verdict: GateVerdict::Approved,
            reviewer_id: "impl-reviewer".to_string(),
        };
        transition(&mut state, input).expect("approve impl review");
        assert_eq!(state.current_phase(), Some(&TddPhase::LintCheck));
    }

    #[test]
    fn impl_review_veto_returns_to_implement() {
        let mut state = fresh_state();
        submit_test(&mut state);
        approve_test_gate(&mut state);
        confirm_red(&mut state);
        state.current_frame_mut().unwrap().phase = TddPhase::ImplReviewGate;

        let reason = VetoReason::try_new("Not narrowest change".to_string())
            .expect("valid reason");
        let input = TddInput::GateVerdicted {
            kind: GateKind::ImplementationReview,
            verdict: GateVerdict::Vetoed { reason },
            reviewer_id: "impl-reviewer".to_string(),
        };
        transition(&mut state, input).expect("veto impl review");
        assert_eq!(state.current_phase(), Some(&TddPhase::Implement));
    }

    #[test]
    fn lint_check_pass_marks_done() {
        let mut state = fresh_state();
        submit_test(&mut state);
        approve_test_gate(&mut state);
        confirm_red(&mut state);
        state.current_frame_mut().unwrap().phase = TddPhase::LintCheck;

        let input = TddInput::CheckResult { passed: true, first_error: None };
        transition(&mut state, input).expect("lint pass");
        assert_eq!(state.current_phase(), Some(&TddPhase::Done));
    }

    #[test]
    fn lint_check_fail_returns_to_implement() {
        let mut state = fresh_state();
        submit_test(&mut state);
        approve_test_gate(&mut state);
        confirm_red(&mut state);
        state.current_frame_mut().unwrap().phase = TddPhase::LintCheck;

        let input = TddInput::CheckResult {
            passed: false,
            first_error: Some("warning: unused import".to_string()),
        };
        transition(&mut state, input).expect("lint fail");
        assert_eq!(state.current_phase(), Some(&TddPhase::Implement));
    }
}
