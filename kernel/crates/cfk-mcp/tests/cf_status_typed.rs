#![expect(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "integration tests use expect/unwrap for assertion clarity"
)]
//! Behavioural test: `cf_status` returns a typed `StatusSummary`.
//!
//! Slice: state_view — cf_status structured output.
//!
//! The conductor must be able to deserialise the response into a typed
//! structure regardless of kernel version, so the shape is part of the public
//! contract — not a raw blob.
//!
//! Scenario: `cf_status_response_round_trips_into_status_summary`
//! — the full response deserialises into `StatusSummary`
//!   (`deny_unknown_fields` makes the contract explicit: any stray field is an error).
//!
//! Expected failure reason (against current production code):
//! serde rejects the response with "unknown field `blocked`" because production
//! emits `blocked` in each phase entry, which `PhaseStatusCounts` (with
//! `deny_unknown_fields`) does not recognise.

use cfk_core::types::ids::ProjectId;
use cfk_core::types::phase::PhaseKind;
use cfk_core::types::routing::WorkType;
use cfk_engine::forge::MemoryForge;
use cfk_mcp::server::{BacklogAddParams, CfkServer, InitParams, PhaseFilterParams};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, RawContent};
use serde::Deserialize;
use tempfile::TempDir;

// ── Semantic response types ───────────────────────────────────────────────────
//
// These are the types the conductor pins to. The production `cf_status`
// handler must emit JSON that round-trips through these exact fields.
//
// `#[serde(deny_unknown_fields)]` is non-negotiable here.
// Without it, serde would silently ignore the production `blocked` field,
// masking the contract violation. With it, any field not listed in the
// struct causes a hard deserialisation error, making every failure mode
// explicit and observable.

/// Per-phase counts in a status dashboard entry.
///
/// `abandoned` is distinct from `done` — items abandoned mid-flight must be
/// surfaced so the conductor can decide whether to requeue them.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PhaseStatusCounts {
    pub phase: PhaseKind,
    /// Items ready to be claimed and worked.
    pub ready: usize,
    /// Items currently claimed and in flight.
    pub in_progress: usize,
    /// Items completed and accepted.
    pub done: usize,
    /// Items that were abandoned before completion.
    pub abandoned: usize,
}

/// Typed status summary — the full response contract for `cf_status`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusSummary {
    /// Project identifier for correlation and display.
    pub project_id: ProjectId,
    /// One entry per phase that has at least one work item.
    pub phases: Vec<PhaseStatusCounts>,
}

// ── Test helpers ──────────────────────────────────────────────────────────────

async fn make_server(dir: &TempDir) -> CfkServer {
    CfkServer::load_with_forge(dir.path().to_path_buf(), MemoryForge::new())
        .await
        .expect("server load should succeed on empty directory")
}

fn is_error(result: &CallToolResult) -> bool {
    result.is_error.unwrap_or(false)
}

fn result_text(result: &CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| {
            if let RawContent::Text(t) = &c.raw {
                Some(t.text.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

// ── Scenario ──────────────────────────────────────────────────────────────────
//
// Given: a project with at least one work item
// When:  cf_status is called
// Then:  the response deserialises into `StatusSummary` without error
//        (`deny_unknown_fields` makes any extra field a hard failure, so the
//         presence of `blocked` OR the absence of `abandoned` both cause panic)
//
// Expected failure reason (against current production code):
// serde rejects the response with "unknown field `blocked`" because production
// emits `blocked` in each phase entry, which `PhaseStatusCounts` (with
// `deny_unknown_fields`) does not recognise.
#[tokio::test]
async fn cf_status_response_round_trips_into_status_summary() {
    let dir = TempDir::new().expect("tempdir");
    let server = make_server(&dir).await;

    // Given: an initialised project
    let init_result = server
        .cf_init(Parameters(InitParams {
            project_root: Some(dir.path().to_str().unwrap().to_string()),
        }))
        .await
        .expect("cf_init should not return McpError");
    assert!(!is_error(&init_result), "cf_init failed: {}", result_text(&init_result));

    // Given: at least one work item so the phases array is non-empty
    let add_result = server
        .cf_backlog_add(Parameters(BacklogAddParams {
            phase: PhaseKind::Development,
            work_type: WorkType::OuterBehavioralTestWriting,
            description: "Some slice".to_string(),
        }))
        .await
        .expect("cf_backlog_add should not return McpError");
    assert!(!is_error(&add_result), "cf_backlog_add failed: {}", result_text(&add_result));

    // When: cf_status is called
    let status_result = server
        .cf_status(Parameters(PhaseFilterParams { phase: None }))
        .await
        .expect("cf_status should not return McpError");
    assert!(
        !is_error(&status_result),
        "cf_status returned a tool error: {}",
        result_text(&status_result)
    );
    let raw = result_text(&status_result);

    // Then: the response deserialises into StatusSummary
    //
    // If production emits `blocked` (which it currently does), serde will
    // reject the JSON here with "unknown field `blocked`" — that is the
    // expected failure mode, visible immediately in the test output.
    let _: StatusSummary = serde_json::from_str(&raw).unwrap_or_else(|e| {
        panic!("cf_status response did not deserialise into StatusSummary: {e}\nRaw: {raw}")
    });
}
