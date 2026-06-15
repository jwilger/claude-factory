#![expect(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "integration tests"
)]
//! Behavioural test: `cf_abandon` marks a work item as Abandoned via the
//! public MCP boundary.
//!
//! Slice: state_change — abandon a work item through the public server API.
//!
//! Scenario: `abandoned_work_item_increments_abandoned_count_in_cf_status`
//!
//! Given:  a project with a Ready work item in the Development backlog
//! When:   `cf_abandon` is called with that `work_item_id`
//! Then:   `cf_status` shows the abandoned count for the Development phase
//!         incremented by 1 (from 0 to 1)
//!
//! Expected failure reason (against current production code):
//! `cf_abandon` does not exist — the call fails to compile because
//! `CfkServer` has no `cf_abandon` method and `AbandonParams` is not defined
//! in `cfk_mcp::server`.

use cfk_core::types::{
    ids::WorkItemId,
    phase::PhaseKind,
    routing::WorkType,
};
use cfk_engine::forge::MemoryForge;
use cfk_mcp::server::{
    AbandonParams, BacklogAddParams, CfkServer, InitParams, PhaseFilterParams,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, RawContent};
use tempfile::TempDir;
use uuid::Uuid;

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
// Given:   an initialised project with one Development work item in Ready status
// When:    `cf_abandon` is called with that work_item_id
// Then:    `cf_status` shows `abandoned == 1` for the Development phase
//
// This test exercises the public boundary exclusively — no internal event
// helpers, no direct store access.  The only observable effect asserted is the
// abandoned count visible through `cf_status`.
#[tokio::test]
async fn abandoned_work_item_increments_abandoned_count_in_cf_status() {
    let dir = TempDir::new().expect("tempdir");
    let server = make_server(&dir).await;

    // ── Given: initialise the project ────────────────────────────────────────

    let init_result = server
        .cf_init(Parameters(InitParams {
            project_root: None,
        }))
        .await
        .expect("cf_init must not return McpError");
    assert!(
        !is_error(&init_result),
        "cf_init returned tool error: {}",
        result_text(&init_result)
    );

    // ── Given: add one Development / OuterBehavioralTestWriting work item ────

    let add_result = server
        .cf_backlog_add(Parameters(BacklogAddParams {
            phase: PhaseKind::Development,
            work_type: WorkType::OuterBehavioralTestWriting,
            description: "Write outer behavioural test for the abandon tool".to_string(),
        }))
        .await
        .expect("cf_backlog_add must not return McpError");
    assert!(
        !is_error(&add_result),
        "cf_backlog_add returned tool error: {}",
        result_text(&add_result)
    );

    // Extract and round-trip the work_item_id through the domain semantic type.
    let add_json: serde_json::Value =
        serde_json::from_str(&result_text(&add_result)).expect("backlog_add response is JSON");
    let raw_id = add_json["work_item_id"]
        .as_str()
        .expect("work_item_id present in backlog_add response");
    let uuid = Uuid::parse_str(raw_id).expect("work_item_id is a valid UUID");
    // Validate through the domain type — proves this is a well-formed WorkItemId.
    let work_item_id: WorkItemId =
        WorkItemId::try_new(uuid).expect("valid WorkItemId");
    // The MCP boundary accepts String; hold the semantic type for proof,
    // convert to String only at the call site.
    let work_item_id_str = work_item_id.to_string();

    // ── When: call cf_abandon with the work_item_id ───────────────────────────

    let abandon_result = server
        .cf_abandon(Parameters(AbandonParams {
            work_item_id: work_item_id_str.clone(),
        }))
        .await
        .expect("cf_abandon must not return McpError");
    assert!(
        !is_error(&abandon_result),
        "cf_abandon returned tool error: {}",
        result_text(&abandon_result)
    );

    // ── Then: cf_status shows abandoned == 1 for Development phase ────────────
    //
    // This is the single behavioral assertion: after a successful cf_abandon,
    // the abandoned count for the affected phase must increment by 1.
    // We query the public cf_status endpoint — no internal state inspection.

    let status_result = server
        .cf_status(Parameters(PhaseFilterParams {
            phase: Some(PhaseKind::Development),
        }))
        .await
        .expect("cf_status must not return McpError");
    assert!(
        !is_error(&status_result),
        "cf_status returned tool error: {}",
        result_text(&status_result)
    );

    let status_raw = result_text(&status_result);
    let status_json: serde_json::Value =
        serde_json::from_str(&status_raw).expect("cf_status response is JSON");

    // Find the Development phase entry and read its abandoned count.
    let phases = status_json["phases"]
        .as_array()
        .expect("cf_status response has a 'phases' array");

    let dev_phase = phases
        .iter()
        .find(|p| p["phase"].as_str() == Some("development"))
        .expect("Development phase entry present after adding and abandoning a Development item");

    let abandoned = dev_phase["abandoned"]
        .as_u64()
        .expect("'abandoned' field is a non-negative integer");

    // Single behavioral assertion: cf_abandon must have moved the item into the
    // Abandoned bucket visible through the public cf_status endpoint.
    assert_eq!(
        abandoned, 1,
        "after cf_abandon, Development phase abandoned count must be 1; \
         got {abandoned}\nFull status response: {status_json}"
    );
}
