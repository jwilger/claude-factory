#![expect(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::indexing_slicing,
    reason = "integration tests use expect/unwrap/indexing for assertion clarity; doc comments use domain names without backticks"
)]
//! Behavioural test: `cf_submit` is rejected when no active lease exists.
//!
//! Slice: state_change — lease guardrail enforcement on cf_submit.
//!
//! Context: the PreToolUse hook only fires for Write/Edit tool calls, so any
//! file mutation performed via Bash (sed, tee, shell redirection, python, etc.)
//! bypasses the lease check.  The fix moves enforcement server-side: `cf_submit`
//! must refuse to advance the state machine when no valid lease is held for the
//! work item, regardless of which client tool performed any preceding writes.
//!
//! Scenario: `cf_submit_without_active_lease_is_rejected`
//! — Given a work item that has been claimed and whose lease has since been
//!   released (simulating a Bash-bypass scenario where no lease is held),
//!   When cf_submit is called for that work item,
//!   Then the tool result must be an error (is_error = true).
//!
//! Expected failure reason (against current production code):
//! `cf_submit` does not check for an active lease; it accepts any valid
//! work_item_id regardless of lease state, so `is_error` is false and the
//! assertion fails.

use cfk_core::types::phase::PhaseKind;
use cfk_core::types::routing::WorkType;
use cfk_engine::forge::MemoryForge;
use cfk_mcp::server::{BacklogAddParams, CfkServer, InitParams, NextStepParams, SubmitParams};
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

async fn make_server_with_session(dir: &TempDir, forge: Arc<MemoryForge>, session: &str) -> CfkServer {
    CfkServer::load_with_forge_and_session(
        dir.path().to_path_buf(),
        forge,
        Some(session.to_string()),
    )
    .await
    .expect("server load should succeed on empty directory")
}

// ── Scenario ──────────────────────────────────────────────────────────────────
//
// Given: a project with a work item that was claimed (lease granted) and then
//        released (simulating the session ending or the lease being dropped —
//        as would happen when a Bash bypass occurs outside any lease context)
// When:  cf_submit is called for that work item with no active lease held
// Then:  the tool result must carry is_error = true
//
// The single behavioral assertion is on line marked "THEN".
#[tokio::test]
async fn cf_submit_without_active_lease_is_rejected() {
    let dir = TempDir::new().expect("tempdir");
    let server = make_server(&dir).await;

    // Given: an initialised project
    let init_result = server
        .cf_init(Parameters(InitParams {
            project_root: Some(dir.path().to_str().unwrap().to_string()),
        }))
        .await
        .expect("cf_init should not return McpError");
    assert!(
        !is_error(&init_result),
        "cf_init failed: {}",
        result_text(&init_result)
    );

    // Given: a development work item exists in the backlog
    let add_result = server
        .cf_backlog_add(Parameters(BacklogAddParams {
            phase: PhaseKind::Development,
            work_type: WorkType::OuterBehavioralTestWriting,
            description: "Harden lease guardrail against Bash bypass".to_string(),
        }))
        .await
        .expect("cf_backlog_add should not return McpError");
    assert!(
        !is_error(&add_result),
        "cf_backlog_add failed: {}",
        result_text(&add_result)
    );

    // Given: the work item was claimed (a lease was granted) by a session
    let next_result = server
        .cf_next_step(Parameters(NextStepParams {
            phase: Some(PhaseKind::Development),
            session_identity: Some("alice".to_string()),
        }))
        .await
        .expect("cf_next_step should not return McpError");
    assert!(
        !is_error(&next_result),
        "cf_next_step failed: {}",
        result_text(&next_result)
    );
    let next_json = result_json(&next_result);
    let work_item_id = next_json["work_item_id"]
        .as_str()
        .expect("cf_next_step response should include work_item_id")
        .to_string();

    // Given: the lease is released (session ends / Bash bypass scenario —
    //        the writer never held a lease recognised by the server)
    let release_result = server
        .cf_release(Parameters(cfk_mcp::server::WorkItemIdParams {
            work_item_id: work_item_id.clone(),
        }))
        .await
        .expect("cf_release should not return McpError");
    assert!(
        !is_error(&release_result),
        "cf_release failed: {}",
        result_text(&release_result)
    );

    // When: cf_submit is called with no active lease for the work item
    let submit_result = server
        .cf_submit(Parameters(SubmitParams {
            work_item_id: work_item_id.clone(),
            result: serde_json::json!({
                "test_content": "// written via Bash, no lease held"
            }),
        }))
        .await
        .expect("cf_submit should not return McpError (tool errors use is_error, not Err)");

    // THEN: the submission must be rejected because no lease is held
    assert!(
        is_error(&submit_result),
        "cf_submit without an active lease must be rejected (is_error = true), \
         but got a success response: {}",
        result_text(&submit_result)
    );
}

/// Scenario: a session that does not hold the lease for a work item is rejected.
///
/// Given: server A (session="alice") claims a work item via cf_next_step.
/// When:  server B (session="bob") sharing the same forge calls cf_submit.
/// Then:  the result must be is_error = true.
#[tokio::test]
async fn cf_submit_from_wrong_session_is_rejected() {
    let dir = TempDir::new().expect("tempdir");
    let forge = MemoryForge::new();

    // Server A: alice's session
    let server_alice =
        make_server_with_session(&dir, Arc::clone(&forge), "alice").await;

    // Initialise project via alice's server.
    let init_result = server_alice
        .cf_init(Parameters(InitParams {
            project_root: Some(dir.path().to_str().unwrap().to_string()),
        }))
        .await
        .expect("cf_init should not return McpError");
    assert!(
        !is_error(&init_result),
        "cf_init failed: {}",
        result_text(&init_result)
    );

    // Add a development work item.
    let add_result = server_alice
        .cf_backlog_add(Parameters(BacklogAddParams {
            phase: PhaseKind::Development,
            work_type: WorkType::OuterBehavioralTestWriting,
            description: "Session-ownership guardrail test".to_string(),
        }))
        .await
        .expect("cf_backlog_add should not return McpError");
    assert!(
        !is_error(&add_result),
        "cf_backlog_add failed: {}",
        result_text(&add_result)
    );

    // Alice claims the work item via cf_next_step (auto-claim).
    let next_result = server_alice
        .cf_next_step(Parameters(NextStepParams {
            phase: Some(PhaseKind::Development),
            session_identity: None, // server already knows session="alice"
        }))
        .await
        .expect("cf_next_step should not return McpError");
    assert!(
        !is_error(&next_result),
        "cf_next_step failed: {}",
        result_text(&next_result)
    );
    let next_json = result_json(&next_result);
    let work_item_id = next_json["work_item_id"]
        .as_str()
        .expect("cf_next_step response should include work_item_id")
        .to_string();

    // Release alice's server so the eventcore-fs store lock is freed before bob
    // opens his own server on the same directory.  Alice's events are persisted
    // to disk and bob will replay them on load.
    drop(server_alice);

    // Server B: bob's session — same TempDir (shares alice's persisted event
    // log), but a different session identity.
    let server_bob =
        make_server_with_session(&dir, Arc::clone(&forge), "bob").await;

    // When: bob submits for alice's work item.
    let submit_result = server_bob
        .cf_submit(Parameters(SubmitParams {
            work_item_id: work_item_id.clone(),
            result: serde_json::json!({
                "test_content": "// attempted by wrong session"
            }),
        }))
        .await
        .expect("cf_submit should not return McpError (tool errors use is_error, not Err)");

    // THEN: the submission must be rejected because bob does not hold the lease.
    assert!(
        is_error(&submit_result),
        "cf_submit from a session that does not hold the lease must be rejected \
         (is_error = true), but got a success response: {}",
        result_text(&submit_result)
    );
}
