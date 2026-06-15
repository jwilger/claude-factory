#![expect(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration tests use expect/unwrap for assertion clarity"
)]
//! Behavioral integration tests for the cfk-mcp server tool handlers.
//!
//! These tests call tool methods directly (no stdio transport). They verify
//! durable effects — event files on disk and `MemoryForge` state — not just
//! return values. No mock libraries; forge double + tempdir only.

use cfk_core::types::forge::{CiStatus, PrPollResult};
use cfk_core::types::phase::PhaseKind;
use cfk_core::types::routing::WorkType;
use cfk_engine::forge::{MemoryForge, PollScript};
use cfk_mcp::server::{
    BacklogAddParams, ClaimParams, CfkServer, GateParams, InitParams, NextStepParams, PrMergeParams,
    PrOpenParams, PrPollParams, SubmitParams,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, RawContent};
use std::sync::Arc;
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────────

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

fn result_json(result: &CallToolResult) -> serde_json::Value {
    let text = result_text(result);
    serde_json::from_str(&text).expect("result content should be valid JSON")
}

async fn make_server(dir: &TempDir) -> CfkServer {
    CfkServer::load_with_forge(dir.path().to_path_buf(), MemoryForge::new())
        .await
        .expect("server load should succeed on empty directory")
}

async fn make_server_with_forge(dir: &TempDir, forge: Arc<MemoryForge>) -> CfkServer {
    CfkServer::load_with_forge(dir.path().to_path_buf(), forge)
        .await
        .expect("server load should succeed on empty directory")
}

async fn init_project(server: &CfkServer, dir: &TempDir) -> serde_json::Value {
    let result = server
        .cf_init(Parameters(InitParams {
            project_root: Some(dir.path().to_str().unwrap().to_string()),
        }))
        .await
        .expect("cf_init should not return McpError");
    assert!(!is_error(&result), "cf_init failed: {}", result_text(&result));
    result_json(&result)
}

// ── cf_init ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn cf_init_creates_project_and_event_file() {
    let dir = TempDir::new().expect("tempdir");
    let server = make_server(&dir).await;

    let json = init_project(&server, &dir).await;

    assert!(json["project_id"].as_str().is_some(), "response should include project_id");
    assert!(json["root"].as_str().is_some(), "response should include root");

    // Durable effect: at least one event file written under .claude-factory/events/v1/
    let event_dir = dir.path().join(".claude-factory").join("events").join("v1");
    let entries: Vec<_> = std::fs::read_dir(&event_dir)
        .expect("event dir should exist")
        .filter_map(std::result::Result::ok)
        .collect();
    assert_eq!(entries.len(), 1, "exactly one ProjectInitialized event should be on disk");
}

#[tokio::test]
async fn cf_init_twice_returns_tool_error() {
    let dir = TempDir::new().expect("tempdir");
    let server = make_server(&dir).await;
    init_project(&server, &dir).await;

    let result = server
        .cf_init(Parameters(InitParams { project_root: None }))
        .await
        .expect("cf_init should not return McpError");
    assert!(is_error(&result), "second cf_init should be a tool error");
}

// ── cf_next_step ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn cf_next_step_on_fresh_project_returns_no_step() {
    let dir = TempDir::new().expect("tempdir");
    let server = make_server(&dir).await;
    init_project(&server, &dir).await;

    let result = server
        .cf_next_step(Parameters(NextStepParams { phase: None, session_identity: None }))
        .await
        .expect("cf_next_step should not return McpError");
    assert!(!is_error(&result), "cf_next_step should succeed: {}", result_text(&result));

    let json = result_json(&result);
    // No work items → no step ready
    assert!(
        json["action"].as_str() == Some("wait") || json["step"].is_null(),
        "fresh project should have no ready step, got: {json}"
    );
}

#[tokio::test]
async fn cf_next_step_without_init_returns_tool_error() {
    let dir = TempDir::new().expect("tempdir");
    let server = make_server(&dir).await;

    let result = server
        .cf_next_step(Parameters(NextStepParams { phase: None, session_identity: None }))
        .await
        .expect("cf_next_step should not return McpError");
    assert!(is_error(&result), "cf_next_step without init should be a tool error");
}

// ── TDD slice: claim → submit test → TestReviewGate → gate → RedCheck ────────

#[tokio::test]
async fn tdd_slice_claim_submit_test_gate_approve() {
    let dir = TempDir::new().expect("tempdir");
    let server = make_server(&dir).await;
    init_project(&server, &dir).await;

    // Add a development work item.
    let add_result = server
        .cf_backlog_add(Parameters(BacklogAddParams {
            phase: PhaseKind::Development,
            work_type: WorkType::OuterBehavioralTestWriting,
            description: "Implement user login".to_string(),
        }))
        .await
        .expect("cf_backlog_add should not return McpError");
    assert!(!is_error(&add_result), "cf_backlog_add failed: {}", result_text(&add_result));

    // cf_next_step with session_identity auto-claims and starts TDD slice.
    let next_result = server
        .cf_next_step(Parameters(NextStepParams {
            phase: Some(PhaseKind::Development),
            session_identity: Some("alice".to_string()),
        }))
        .await
        .expect("cf_next_step should not return McpError");
    assert!(!is_error(&next_result), "cf_next_step failed: {}", result_text(&next_result));
    let next_json = result_json(&next_result);
    let work_item_id = next_json["work_item_id"]
        .as_str()
        .expect("cf_next_step response should include work_item_id")
        .to_string();

    // Submit test code — advances to TestReviewGate.
    let submit_result = server
        .cf_submit(Parameters(SubmitParams {
            work_item_id: work_item_id.clone(),
            result: serde_json::json!({ "test_content": "#[test] fn it_works() { assert!(true); }" }),
        }))
        .await
        .expect("cf_submit should not return McpError");
    assert!(!is_error(&submit_result), "cf_submit failed: {}", result_text(&submit_result));
    let submit_json = result_json(&submit_result);
    assert_eq!(
        submit_json["advanced_to"].as_str(),
        Some("test_review_gate"),
        "after test submit should be at TestReviewGate, got: {submit_json}"
    );

    // Gate: reviewer_id must differ from author.
    let gate_result = server
        .cf_gate(Parameters(GateParams {
            work_item_id: work_item_id.clone(),
            reviewer_id: "bob".to_string(),
            verdict: "approved".to_string(),
            reason: None,
        }))
        .await
        .expect("cf_gate should not return McpError");
    assert!(!is_error(&gate_result), "cf_gate failed: {}", result_text(&gate_result));
    let gate_json = result_json(&gate_result);
    assert_eq!(
        gate_json["advanced_to"].as_str(),
        Some("red_check"),
        "after test gate approval should be at RedCheck, got: {gate_json}"
    );
}

// ── PR lifecycle via scripted MemoryForge ─────────────────────────────────────

#[tokio::test]
async fn pr_poll_merge_lifecycle() {
    let dir = TempDir::new().expect("tempdir");
    let forge = MemoryForge::new();

    let server = make_server_with_forge(&dir, Arc::clone(&forge)).await;
    init_project(&server, &dir).await;

    // Add a review-phase work item.
    let add_result = server
        .cf_backlog_add(Parameters(BacklogAddParams {
            phase: PhaseKind::Review,
            work_type: WorkType::PrCommentTriage,
            description: "Review PR for user login".to_string(),
        }))
        .await
        .expect("cf_backlog_add");
    assert!(!is_error(&add_result));
    let add_json = result_json(&add_result);
    let work_item_id = add_json["work_item_id"]
        .as_str()
        .expect("backlog_add should return work_item_id")
        .to_string();

    // Claim the review item.
    let claim_result = server
        .cf_claim(Parameters(ClaimParams {
            work_item_id: work_item_id.clone(),
            session_identity: "alice".to_string(),
        }))
        .await
        .expect("cf_claim");
    assert!(!is_error(&claim_result), "cf_claim failed: {}", result_text(&claim_result));

    // Open PR — MemoryForge assigns number 1.
    let pr_open_result = server
        .cf_pr_open(Parameters(PrOpenParams {
            work_item_id: work_item_id.clone(),
            title: "feat: user login".to_string(),
            body: "Implements login flow".to_string(),
            head: "feature/login".to_string(),
            base: "main".to_string(),
        }))
        .await
        .expect("cf_pr_open");
    assert!(!is_error(&pr_open_result), "cf_pr_open failed: {}", result_text(&pr_open_result));
    let pr_json = result_json(&pr_open_result);
    assert!(pr_json["pr_number"].as_u64().is_some(), "pr_open should return pr_number");

    // Pre-load poll script: first pending, then passing + approved.
    forge.set_poll_script(
        1,
        PollScript::new(vec![
            PrPollResult { ci_status: CiStatus::Pending, approved: false, comments: Vec::new() },
            PrPollResult { ci_status: CiStatus::Passing, approved: true, comments: Vec::new() },
        ]),
    );

    // First poll: CI pending, no AllGreen yet.
    let poll1 = server
        .cf_pr_poll(Parameters(PrPollParams { work_item_id: work_item_id.clone() }))
        .await
        .expect("cf_pr_poll 1");
    assert!(!is_error(&poll1), "first poll failed: {}", result_text(&poll1));
    let poll1_json = result_json(&poll1);
    assert_ne!(
        poll1_json["all_green"].as_bool(),
        Some(true),
        "first poll should not be all_green yet, got: {poll1_json}"
    );

    // Second poll: CI passing + approved → AllGreen.
    let poll2 = server
        .cf_pr_poll(Parameters(PrPollParams { work_item_id: work_item_id.clone() }))
        .await
        .expect("cf_pr_poll 2");
    assert!(!is_error(&poll2), "second poll failed: {}", result_text(&poll2));
    let poll2_json = result_json(&poll2);
    assert_eq!(
        poll2_json["all_green"].as_bool(),
        Some(true),
        "second poll should be all_green, got: {poll2_json}"
    );

    // Merge.
    let merge_result = server
        .cf_pr_merge(Parameters(PrMergeParams { work_item_id: work_item_id.clone() }))
        .await
        .expect("cf_pr_merge");
    assert!(!is_error(&merge_result), "cf_pr_merge failed: {}", result_text(&merge_result));

    // Durable effect: MemoryForge recorded the merge.
    assert!(forge.is_merged(1), "MemoryForge should record PR 1 as merged");
}

// ── Invalid parameter handling ────────────────────────────────────────────────

#[tokio::test]
async fn invalid_work_item_uuid_returns_mcp_error() {
    let dir = TempDir::new().expect("tempdir");
    let server = make_server(&dir).await;
    init_project(&server, &dir).await;

    // cf_claim with a non-UUID work_item_id should return McpError (invalid_params).
    let result = server
        .cf_claim(Parameters(ClaimParams {
            work_item_id: "not-a-uuid".to_string(),
            session_identity: "alice".to_string(),
        }))
        .await;
    assert!(result.is_err(), "non-UUID work_item_id should return McpError");
}

#[test]
fn invalid_phase_string_rejected_at_deserialization() {
    // BacklogAddParams.phase is now PhaseKind; serde rejects unknown variants
    // before the handler ever runs — no server needed.
    let json = r#"{"phase":"nonexistent_phase","work_type":"outer_behavioral_test_writing","description":"test"}"#;
    let result = serde_json::from_str::<BacklogAddParams>(json);
    assert!(result.is_err(), "unknown phase should fail JSON deserialization");
}

#[tokio::test]
async fn gate_with_same_author_and_reviewer_returns_error() {
    let dir = TempDir::new().expect("tempdir");
    let server = make_server(&dir).await;
    init_project(&server, &dir).await;

    // Add and auto-claim a development item.
    server
        .cf_backlog_add(Parameters(BacklogAddParams {
            phase: PhaseKind::Development,
            work_type: WorkType::OuterBehavioralTestWriting,
            description: "Some work".to_string(),
        }))
        .await
        .expect("cf_backlog_add");

    let next = server
        .cf_next_step(Parameters(NextStepParams {
            phase: Some(PhaseKind::Development),
            session_identity: Some("alice".to_string()),
        }))
        .await
        .expect("cf_next_step");
    let work_item_id = result_json(&next)["work_item_id"]
        .as_str()
        .expect("work_item_id")
        .to_string();

    // Submit test to reach TestReviewGate.
    server
        .cf_submit(Parameters(SubmitParams {
            work_item_id: work_item_id.clone(),
            result: serde_json::json!({ "test_content": "#[test] fn it_works() {}" }),
        }))
        .await
        .expect("cf_submit");

    // Gate with same reviewer as author should fail.
    let gate_result = server
        .cf_gate(Parameters(GateParams {
            work_item_id,
            reviewer_id: "alice".to_string(), // same as author
            verdict: "approved".to_string(),
            reason: None,
        }))
        .await;
    assert!(
        gate_result.is_err() || is_error(gate_result.as_ref().unwrap()),
        "same author/reviewer gate should fail"
    );
}
