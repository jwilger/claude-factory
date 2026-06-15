#![expect(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "integration tests"
)]
//! Behavioural test: `cf_adr_submit` is rejected when the work item is Done.
//!
//! Slice: state_change — ADR state machine guards completed work items.
//!
//! Bug observed 2026-06-12: after `adr_decided(accepted=false)` immediately
//! emitted `work_item_completed`, a revised `cf_adr_submit` call was accepted
//! against the completed item and orphaned with no review gate ever scheduled.
//!
//! Scenario: `cf_adr_submit_on_completed_work_item_is_rejected`
//!
//! Given:  a project with an Architecture / AdrDrafting work item that has
//!         been driven all the way to Done (ADR accepted, work item completed)
//! When:   `cf_adr_submit` is called a second time on that same work_item_id
//! Then:   the tool result must carry `is_error = true`
//!
//! Expected failure reason (against current production code):
//! `cf_adr_submit` does not check whether the target work item is in Done
//! status, so the second submission succeeds (`is_error = false`) and the
//! assertion fails.

use cfk_core::types::{
    gate::GateVerdict,
    ids::WorkItemId,
    lease::SessionIdentity,
    phase::PhaseKind,
    routing::WorkType,
    tdd::ReviewerId,
};
use cfk_engine::forge::MemoryForge;
use cfk_mcp::server::{
    AdrSubmitParams, BacklogAddParams, CfkServer, ClaimParams, GateParams, InitParams,
    NextStepParams,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, RawContent};
use tempfile::TempDir;
use uuid::Uuid;

// ── Test helpers ───────────────────────────────────────────────────────────────

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

/// Advance the server to a state where the Architecture work item is Done
/// (ADR accepted, work item completed) and return the `work_item_id`.
///
/// Uses `expect`/`unwrap` only to guard against tool errors that would make
/// the final assertion meaningless.  No assertions are made on intermediate
/// action types — this is setup, not the scenario under test.
///
/// Steps performed:
///   1. `cf_init`               — initialise the project
///   2. `cf_backlog_add`        — add one Architecture / AdrDrafting work item
///   3. `cf_next_step`          — kernel schedules the architect agent
///   4. `cf_claim`              — lease the item as "session-alpha"
///   5. `cf_adr_submit`         — architect submits the ADR draft
///   6. `cf_next_step`          — kernel schedules the gate reviewer
///   7. `cf_gate` (approved)    — reviewer accepts the ADR → work item Done
async fn advance_adr_to_accepted(server: &CfkServer) -> String {
    // ── Step 1: initialise the project ────────────────────────────────────────

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

    // ── Step 2: add an Architecture / AdrDrafting work item ──────────────────

    let add_result = server
        .cf_backlog_add(Parameters(BacklogAddParams {
            phase: PhaseKind::Architecture,
            work_type: WorkType::AdrDrafting,
            description: "Choose the primary event store technology".to_string(),
        }))
        .await
        .expect("cf_backlog_add must not return McpError");
    assert!(
        !is_error(&add_result),
        "cf_backlog_add returned tool error: {}",
        result_text(&add_result)
    );

    // Extract and validate the work_item_id through the domain semantic type.
    let add_json: serde_json::Value =
        serde_json::from_str(&result_text(&add_result)).expect("backlog_add response is JSON");
    let raw_id = add_json["work_item_id"]
        .as_str()
        .expect("work_item_id present in backlog_add response");
    let uuid = Uuid::parse_str(raw_id).expect("work_item_id is a valid UUID");
    // Validate through the domain type — proves this is a well-formed WorkItemId.
    let work_item_id: WorkItemId =
        WorkItemId::try_new(uuid).expect("valid WorkItemId");
    let work_item_id_str = work_item_id.to_string();

    // Validate the reviewer identity through the domain type before passing it
    // to the String MCP boundary.
    let reviewer_id: ReviewerId =
        ReviewerId::try_new("reviewer-gamma".to_string()).expect("valid ReviewerId");

    // Validate the session identity through the domain type before passing it
    // to the String MCP boundary.
    let session_id = SessionIdentity::try_new("session-alpha".to_string())
        .expect("valid SessionIdentity");

    // Derive the verdict string from the domain type to avoid raw literals.
    let verdict = GateVerdict::Approved;
    let verdict_str = format!("{verdict:?}").to_lowercase();

    // ── Step 3: cf_next_step — kernel schedules the architect agent ───────────

    let next1_result = server
        .cf_next_step(Parameters(NextStepParams {
            phase: Some(PhaseKind::Architecture),
            session_identity: None,
        }))
        .await
        .expect("cf_next_step (1) must not return McpError");
    assert!(
        !is_error(&next1_result),
        "cf_next_step (1) returned tool error: {}",
        result_text(&next1_result)
    );

    // ── Step 4: cf_claim — lease the item as session-alpha ───────────────────

    let claim_result = server
        .cf_claim(Parameters(ClaimParams {
            work_item_id: work_item_id_str.clone(),
            session_identity: session_id.to_string(),
        }))
        .await
        .expect("cf_claim must not return McpError");
    assert!(
        !is_error(&claim_result),
        "cf_claim returned tool error: {}",
        result_text(&claim_result)
    );

    // ── Step 5: cf_adr_submit — architect submits the ADR draft ──────────────

    let submit1_result = server
        .cf_adr_submit(Parameters(AdrSubmitParams {
            work_item_id: work_item_id_str.clone(),
            title: "Adopt EventCore 0.9 as the kernel event store".to_string(),
            content: "## Context\nWe need a durable, auditable event store.\n\
                      ## Decision\nAdopt EventCore 0.9 with eventcore-fs.\n\
                      ## Consequences\nDurable history; replay capability; \
                      couples the kernel to the EventCore API surface."
                .to_string(),
        }))
        .await
        .expect("cf_adr_submit (1) must not return McpError");
    assert!(
        !is_error(&submit1_result),
        "cf_adr_submit (1) returned tool error: {}",
        result_text(&submit1_result)
    );

    // ── Step 6: cf_next_step — kernel schedules the gate reviewer ─────────────

    let next2_result = server
        .cf_next_step(Parameters(NextStepParams {
            phase: Some(PhaseKind::Architecture),
            session_identity: None,
        }))
        .await
        .expect("cf_next_step (2) must not return McpError");
    assert!(
        !is_error(&next2_result),
        "cf_next_step (2) returned tool error: {}",
        result_text(&next2_result)
    );

    // ── Step 7: cf_gate (approved) — reviewer accepts the ADR → Done ──────────

    let gate_result = server
        .cf_gate(Parameters(GateParams {
            work_item_id: work_item_id_str.clone(),
            reviewer_id: reviewer_id.to_string(),
            verdict: verdict_str,
            reason: None,
        }))
        .await
        .expect("cf_gate must not return McpError");
    assert!(
        !is_error(&gate_result),
        "cf_gate returned tool error: {}",
        result_text(&gate_result)
    );

    work_item_id_str
}

// ── Scenario ───────────────────────────────────────────────────────────────────
//
// Given:   an Architecture work item that has reached Done status
//          (ADR accepted by reviewer, work item completed)
// When:    cf_adr_submit is called again on the same work_item_id
// Then:    the tool result must carry is_error = true
//
// Expected failure: current production does not guard cf_adr_submit against
// Done work items, so the second submission succeeds (is_error = false) and
// the assertion fails.
#[tokio::test]
async fn cf_adr_submit_on_completed_work_item_is_rejected() {
    let dir = TempDir::new().expect("tempdir");
    let server = make_server(&dir).await;

    // Advance to Done state using the setup helper.
    let work_item_id_str = advance_adr_to_accepted(&server).await;

    // Validate the returned id is still a well-formed WorkItemId before reuse.
    let uuid = Uuid::parse_str(&work_item_id_str).expect("work_item_id from setup is a valid UUID");
    let work_item_id: WorkItemId =
        WorkItemId::try_new(uuid).expect("work_item_id from setup is a valid WorkItemId");

    // When: a second cf_adr_submit call is made against the now-Done work item.
    let submit2_result = server
        .cf_adr_submit(Parameters(AdrSubmitParams {
            work_item_id: work_item_id.to_string(),
            title: "Revised: Adopt EventCore 0.9 (second attempt)".to_string(),
            content: "## Context\nThe first ADR was already accepted.\n\
                      ## Decision\nThis submission should be rejected.\n\
                      ## Consequences\nIf accepted, the work item is orphaned \
                      with no review gate ever scheduled."
                .to_string(),
        }))
        .await
        .expect("cf_adr_submit (2) must not return McpError (tool errors use is_error, not Err)");

    // Then: the submission must be rejected because the work item is Done.
    assert!(
        is_error(&submit2_result),
        "cf_adr_submit against a completed (Done) work item must be rejected \
         (is_error = true), but got a success response: {}",
        result_text(&submit2_result)
    );
}
