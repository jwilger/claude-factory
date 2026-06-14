#![expect(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "integration tests"
)]
//! Behavioural test: ADR reviewer veto surfaces as `ask_human`.
//!
//! Slice: state_change — gate semantics for non-development (architecture) gates.
//!
//! Scenario: `adr_reviewer_veto_surfaces_as_ask_human`
//!
//! Given:  a project with an architecture work item in `AdrDrafting`
//! When:   the architect submits an ADR, the reviewer vetoes it via `cf_gate`,
//!         and the conductor calls `cf_next_step`
//! Then:   `cf_next_step` returns a step whose `action.type` is `"ask_human"`
//!         (not `"spawn_agent"`, `"gate_review"`, or an idle response)
//!
//! Expected failure reason (against current production code):
//! After `cf_gate` with verdict=`"vetoed"`, the kernel marks the work item Done
//! and emits `WorkItemCompleted`, so `cf_next_step` returns `{"status":"idle",...}`
//! rather than `{"status":"ready","action":{"type":"ask_human",...}}`.
//! The assertion `action_type == "ask_human"` therefore fails.

use cfk_core::types::{
    ids::WorkItemId,
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

// ── Test helpers ──────────────────────────────────────────────────────────────

fn make_server(dir: &TempDir) -> CfkServer {
    CfkServer::load_with_forge(dir.path().to_path_buf(), MemoryForge::new())
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

/// Advance the server through the full veto sequence without asserting
/// intermediate action types.  Uses `expect`/`unwrap` only to guard against
/// tool errors that would make the subsequent assertion meaningless.
///
/// Steps performed:
///   1. `cf_init`             — initialise the project
///   2. `cf_backlog_add`      — add one Architecture / AdrDrafting work item
///   3. `cf_next_step`        — let the kernel schedule the architect agent
///   4. `cf_claim`            — lease the item as "session-alpha"
///   5. `cf_adr_submit`       — submit the ADR draft (enters PendingReview)
///   6. `cf_next_step`        — let the kernel schedule the gate reviewer
///   7. `cf_gate` (vetoed)    — reviewer-beta rejects the ADR
///
/// Returns the `work_item_id` string that was used throughout (for completeness;
/// the caller does not need it for the final assertion).
async fn setup_adr_vetoed_state(server: &CfkServer) -> String {
    // ── Step 1: initialise the project ───────────────────────────────────────

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
            description: "Choose an event store technology".to_string(),
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
    // Validate through the domain type — proves this is a well-formed WorkItemId,
    // not an arbitrary string.
    let work_item_id: WorkItemId =
        WorkItemId::try_new(uuid).expect("valid WorkItemId");
    // The MCP boundary accepts String; we hold the semantic type for proof and
    // convert to String only at the call sites.
    let work_item_id_str = work_item_id.to_string();

    // Validate the reviewer identity through the domain type as well.
    // ReviewerId validates non-empty; holding this proves "reviewer-beta" is a
    // well-formed reviewer identity before we hand it to the String boundary.
    let reviewer_id: ReviewerId =
        ReviewerId::try_new("reviewer-beta".to_string()).expect("valid ReviewerId");

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
    // No assertion on action type — setup only guards against errors.

    // ── Step 4: cf_claim — lease the item as session-alpha ───────────────────

    let claim_result = server
        .cf_claim(Parameters(ClaimParams {
            work_item_id: work_item_id_str.clone(),
            session_identity: "session-alpha".to_string(),
        }))
        .await
        .expect("cf_claim must not return McpError");
    assert!(
        !is_error(&claim_result),
        "cf_claim returned tool error: {}",
        result_text(&claim_result)
    );

    // ── Step 5: cf_adr_submit — ADR draft enters PendingReview ───────────────

    let submit_result = server
        .cf_adr_submit(Parameters(AdrSubmitParams {
            work_item_id: work_item_id_str.clone(),
            title: "Use EventCore as the kernel event store".to_string(),
            content: "## Context\nWe need a durable event store.\n\
                      ## Decision\nAdopt EventCore 0.9 with eventcore-fs.\n\
                      ## Consequences\n(section intentionally sparse for this test)"
                .to_string(),
        }))
        .await
        .expect("cf_adr_submit must not return McpError");
    assert!(
        !is_error(&submit_result),
        "cf_adr_submit returned tool error: {}",
        result_text(&submit_result)
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
    // No assertion on action type — setup only guards against errors.

    // ── Step 7: cf_gate — reviewer-beta vetoes the ADR ───────────────────────

    let gate_result = server
        .cf_gate(Parameters(GateParams {
            work_item_id: work_item_id_str.clone(),
            // Use the validated semantic type at the String boundary.
            reviewer_id: reviewer_id.to_string(),
            verdict: "vetoed".to_string(),
            reason: Some("Missing consequences section".to_string()),
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

// ── Scenario ──────────────────────────────────────────────────────────────────
//
// Given:   an initialised project with one Architecture / AdrDrafting work item
// When:    the architect submits an ADR and the reviewer vetoes it
//          (setup via `setup_adr_vetoed_state`)
//          and the conductor calls `cf_next_step`
// Then:    `cf_next_step` returns action.type = "ask_human"
//          (a human must decide whether to requeue, escalate, or abandon the ADR)
//
// Expected failure: current production marks the work item Done on veto and
// returns status=idle (no ready work), so `action_type == "ask_human"` panics.
#[tokio::test]
async fn adr_reviewer_veto_surfaces_as_ask_human() {
    let dir = TempDir::new().expect("tempdir");
    let server = make_server(&dir);

    // Advance to the post-veto state using the setup helper.
    setup_adr_vetoed_state(&server).await;

    // ── Then: cf_next_step must return ask_human ──────────────────────────────
    //
    // A human decision is required after a reviewer vetoes an ADR:
    // the conductor cannot autonomously decide whether to requeue the architect,
    // escalate the rejection, or abandon the ADR.  Looping back to spawn_agent
    // would let an unbounded retry chain form without human oversight.

    let next3_result = server
        .cf_next_step(Parameters(NextStepParams {
            phase: Some(PhaseKind::Architecture),
            session_identity: None,
        }))
        .await
        .expect("cf_next_step (final) must not return McpError");
    assert!(
        !is_error(&next3_result),
        "cf_next_step (final) returned tool error: {}",
        result_text(&next3_result)
    );

    let next3_raw = result_text(&next3_result);
    let next3_json: serde_json::Value =
        serde_json::from_str(&next3_raw).expect("cf_next_step (final) response is JSON");

    let status = next3_json["status"].as_str().unwrap_or("<missing>");
    let action_type = next3_json["action"]["type"].as_str().unwrap_or("<missing>");

    // Single behavioral assertion: after an ADR reviewer veto, the kernel must
    // surface an ask_human action so a human can decide next steps.
    assert_eq!(
        action_type, "ask_human",
        "after ADR reviewer veto, cf_next_step must return action.type=ask_human \
         (a human must decide whether to requeue, escalate, or abandon the ADR); \
         got status={status:?}, action.type={action_type:?}\nFull response: {next3_json}"
    );
}
