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
/// Recognises rustc `error[E…]` lines, cargo `FAILED` lines, and generic
/// `error:` / `Error:` prefixes.  Falls back to the first non-blank line if
/// nothing more specific is found.
#[must_use]
pub fn extract_first_error(output: &str) -> Option<String> {
    // Priority 1: rustc structured errors (e.g. `error[E0308]: mismatched types`)
    if let Some(line) = output.lines().find(|l| {
        let t = l.trim_start();
        t.starts_with("error[E") || t.starts_with("error[W")
    }) {
        return Some(line.trim().to_string());
    }

    // Priority 2: cargo test failure lines
    if let Some(line) = output.lines().find(|l| {
        let t = l.trim_start();
        t.starts_with("FAILED") || t.contains("test result: FAILED")
    }) {
        return Some(line.trim().to_string());
    }

    // Priority 3: generic error/Error prefix
    if let Some(line) = output.lines().find(|l| {
        let t = l.trim_start();
        t.starts_with("error:") || t.starts_with("Error:")
    }) {
        return Some(line.trim().to_string());
    }

    // Fallback: first non-blank line
    output.lines().find(|l| !l.trim().is_empty()).map(|l| l.trim().to_string())
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
