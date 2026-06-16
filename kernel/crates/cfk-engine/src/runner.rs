//! Deterministic check runner.
//!
//! The kernel runs all checks itself; agents never self-report pass/fail.
//! This module executes shell commands and extracts structured results.

use std::{io, path::Path};
use std::process::Stdio;
use thiserror::Error;
use tokio::process::Command;

/// Error returned when a check cannot be spawned or its output captured.
#[derive(Debug, Error)]
#[error("failed to run check: {0}")]
pub struct RunnerError(#[from] io::Error);

/// The result of running a configured check.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub passed: bool,
    /// The first meaningful error or warning line from the output.
    pub first_error: Option<String>,
    /// Full combined stdout + stderr output (truncated to 64 KiB).
    pub full_output: String,
}

const MAX_OUTPUT_BYTES: usize = 65_536;

/// Run `command` as a shell command in `working_dir` and return a
/// structured `CheckResult`.
///
/// # Errors
/// Returns an error if the process cannot be spawned or its output cannot
/// be captured.
pub async fn run_check(command: &str, working_dir: &Path) -> Result<CheckResult, RunnerError> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    let full_output = if combined.len() > MAX_OUTPUT_BYTES {
        format!("{}... [truncated]", &combined[..combined.floor_char_boundary(MAX_OUTPUT_BYTES)])
    } else {
        combined
    };

    let passed = output.status.success();
    let first_error = if passed {
        None
    } else {
        extract_first_error(&full_output)
    };

    Ok(CheckResult { passed, first_error, full_output })
}

/// Extract the first meaningful error line from compiler / test output.
///
/// Prefers the most *actionable* detail over summary lines, so the implementer
/// agent is handed the real cause (e.g. `ImportError: cannot import name 'x'` or
/// `AssertionError: 250 != 175`) rather than a framework summary like
/// `FAILED (errors=1)`. Recognises, in priority order: rustc structured errors,
/// Rust panics, typed exceptions / `error:` prefixes (Python, JS, generic),
/// pytest assertion (`E   …`) lines, then the cargo/unittest `FAILED` summary as
/// a fallback, and finally the first non-blank line.
#[must_use]
pub fn extract_first_error(output: &str) -> Option<String> {
    let find = |pred: &dyn Fn(&str) -> bool| {
        output.lines().find(|l| pred(l.trim())).map(|l| l.trim().to_string())
    };

    // 1: rustc structured errors (e.g. `error[E0308]: mismatched types`)
    if let Some(line) = find(&|t| t.starts_with("error[E") || t.starts_with("error[W")) {
        return Some(line);
    }
    // 2: Rust panics (the failing assertion site)
    if let Some(line) = find(&|t| t.contains("panicked at")) {
        return Some(line);
    }
    // 3: typed exceptions / error prefixes — the actionable cause across most
    //    languages (`ImportError: …`, `AssertionError: …`, `error: …`). Require
    //    the marker at the line's first token so a benign mid-sentence
    //    "… Error: handled" doesn't shadow a later real error.
    if let Some(line) = find(&is_typed_error) {
        return Some(line);
    }
    // 4: pytest assertion detail lines (`E   assert 1 == 2`)
    if let Some(line) = find(&|t| t.starts_with("E   ")) {
        return Some(line);
    }
    // 5: cargo/unittest failure summary (fallback when no detail line was found)
    if let Some(line) = find(&|t| t.starts_with("FAILED") || t.contains("test result: FAILED")) {
        return Some(line);
    }
    // 6: first non-blank line
    find(&|t| !t.is_empty())
}

/// Whether a trimmed line names a typed error at its first token — `error:`,
/// `<Word>Error:`, or `<Word>Exception:` (e.g. `ImportError:`, `AssertionError:`)
/// — as opposed to merely mentioning "Error:" mid-sentence.
fn is_typed_error(t: &str) -> bool {
    if t.starts_with("error:") {
        return true;
    }
    let first = t.split_whitespace().next().unwrap_or("");
    first.ends_with("Error:") || first.ends_with("Exception:")
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "test functions use expect for assertion clarity"
)]
mod tests {
    use super::*;

    #[test]
    fn extracts_rustc_error_line() {
        let output = "warning: unused import\nerror[E0308]: mismatched types\n  --> src/lib.rs:5:10\n";
        assert_eq!(
            extract_first_error(output),
            Some("error[E0308]: mismatched types".to_string())
        );
    }

    #[test]
    fn extracts_generic_error_line() {
        let output = "some preamble\nerror: something went wrong\nmore output\n";
        assert_eq!(
            extract_first_error(output),
            Some("error: something went wrong".to_string())
        );
    }

    #[test]
    fn prefers_typed_exception_over_failed_summary() {
        // Python unittest: the real cause is the ImportError, not the summary line.
        let output = "E\n\
             ======================================================================\n\
             ERROR: test_zero (test_late_fee.LateFeeTest)\n\
             Traceback (most recent call last):\n  \
               File \"tests/test_late_fee.py\", line 2, in <module>\n    \
                 from src.late_fee import late_fee\n\
             ImportError: cannot import name 'late_fee'\n\
             ----------------------------------------------------------------------\n\
             Ran 1 test in 0.001s\n\nFAILED (errors=1)\n";
        assert_eq!(
            extract_first_error(output),
            Some("ImportError: cannot import name 'late_fee'".to_string())
        );
    }

    #[test]
    fn extracts_assertion_error_over_summary() {
        let output = "AssertionError: 250 != 175\nFAILED (failures=1)\n";
        assert_eq!(
            extract_first_error(output),
            Some("AssertionError: 250 != 175".to_string())
        );
    }

    #[test]
    fn ignores_benign_mid_sentence_error_mention() {
        let output = "note: Previous Error: handled gracefully\nerror: real compile failure\n";
        assert_eq!(
            extract_first_error(output),
            Some("error: real compile failure".to_string())
        );
    }

    #[test]
    fn extracts_pytest_assertion_line() {
        let output = "tests/test_x.py::test_a FAILED\nE   assert 1 == 2\n";
        assert_eq!(extract_first_error(output), Some("E   assert 1 == 2".to_string()));
    }

    #[test]
    fn falls_back_to_failed_summary_when_no_detail() {
        let output = "running 3 tests\ntest result: FAILED. 2 passed; 1 failed\n";
        assert_eq!(
            extract_first_error(output),
            Some("test result: FAILED. 2 passed; 1 failed".to_string())
        );
    }

    #[test]
    fn extracts_rust_panic_line() {
        let output = "running 1 test\nthread 'it' panicked at src/lib.rs:5:9:\nassertion failed\n";
        assert_eq!(
            extract_first_error(output),
            Some("thread 'it' panicked at src/lib.rs:5:9:".to_string())
        );
    }

    #[test]
    fn falls_back_to_first_non_blank_line() {
        let output = "\n\ncustom failure message\nmore stuff\n";
        assert_eq!(
            extract_first_error(output),
            Some("custom failure message".to_string())
        );
    }

    #[tokio::test]
    async fn run_passing_check() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result = run_check("true", dir.path()).await.expect("run_check");
        assert!(result.passed);
        assert!(result.first_error.is_none());
    }

    #[tokio::test]
    async fn run_failing_check() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result = run_check("echo 'error: boom'; exit 1", dir.path())
            .await
            .expect("run_check");
        assert!(!result.passed);
        assert!(result.first_error.is_some());
    }
}
