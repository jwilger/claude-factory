#![expect(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "integration tests"
)]
//! Behavioural test: the `adr_review` gate must NOT route to the Codex model
//! "o4-mini".
//!
//! Slice: state_view — routing table configuration for the adr_review gate.
//!
//! Background: "o4-mini" is rejected by the Codex CLI under a ChatGPT account.
//! The default routing table must specify a supported model (e.g. "gpt-5.5")
//! OR the routing table must allow per-install configuration so operators can
//! override the unsupported default.
//!
//! Scenario: `adr_review_gate_does_not_route_to_o4_mini`
//!
//! Given:  a project initialised with the default routing table,
//!         with one Architecture / AdrDrafting work item in PendingReview
//!         (i.e. the architect has submitted an ADR draft and is awaiting review)
//! When:   `cf_next_step` is called and returns a step for the adr_review gate
//! Then:   the step must NOT route to the Codex model "o4-mini"
//!         (a GateReview step with a Claude executor OR a Codex executor with any
//!         model other than "o4-mini" both satisfy the constraint)
//!
//! Expected failure reason (against current production code):
//! The default routing table in `cfk-engine/src/config.rs` hardcodes
//! `codex("o4-mini", CodexEffort::High)` for `WorkType::AdrReview`, so the
//! gate_review step carries `executor = ExecutorSpec::Codex { model: "o4-mini", .. }` and the
//! assertion fails.

use cfk_core::types::{
    phase::PhaseKind,
    routing::{CodexModel, ExecutorSpec, WorkType},
    step::{ReadyStep, StepAction},
};
use cfk_engine::forge::MemoryForge;
use cfk_mcp::server::{
    AdrSubmitParams, BacklogAddParams, CfkServer, ClaimParams, InitParams, NextStepParams,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, RawContent};
use tempfile::TempDir;

// ── Test helpers ───────────────────────────────────────────────────────────────

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

/// Advance the server to the state where the kernel has scheduled the step
/// following ADR submission, and return it as a typed `ReadyStep`.
///
/// Uses `expect`/`unwrap` only to guard against tool errors that would make
/// the final assertion meaningless.  Does NOT assert on step type — that is
/// the concern of the scenario assertion, not the setup.
///
/// Steps performed:
///   1. `cf_init`         — initialise the project
///   2. `cf_backlog_add`  — add one Architecture / AdrDrafting work item
///   3. `cf_next_step`    — kernel schedules the architect agent (spawn_agent)
///   4. `cf_claim`        — lease the item
///   5. `cf_adr_submit`   — architect submits the ADR draft (→ PendingReview)
///   6. `cf_next_step`    — kernel schedules the next step (expected: gate_review)
///
/// Returns the typed step produced in step 6.
async fn advance_to_adr_review_state(server: &CfkServer) -> ReadyStep {
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

    // ── Step 2: add one Architecture / AdrDrafting work item ─────────────────

    let add_result = server
        .cf_backlog_add(Parameters(BacklogAddParams {
            phase: PhaseKind::Architecture,
            work_type: WorkType::AdrDrafting,
            description: "Choose the primary persistence mechanism".to_string(),
        }))
        .await
        .expect("cf_backlog_add must not return McpError");
    assert!(
        !is_error(&add_result),
        "cf_backlog_add returned tool error: {}",
        result_text(&add_result)
    );

    // Extract the work_item_id string from the response.
    // ClaimParams.work_item_id and AdrSubmitParams.work_item_id are String at the
    // MCP wire boundary; no domain-type wrapping is possible here.
    let add_json: serde_json::Value =
        serde_json::from_str(&result_text(&add_result)).expect("backlog_add response is JSON");
    let work_item_id_str = add_json["work_item_id"]
        .as_str()
        .expect("work_item_id present in backlog_add response")
        .to_string();

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

    // ── Step 4: cf_claim — lease the item ────────────────────────────────────

    // ClaimParams.session_identity is String at the MCP wire boundary.
    let claim_result = server
        .cf_claim(Parameters(ClaimParams {
            work_item_id: work_item_id_str.clone(),
            session_identity: "session-adr-author".to_string(),
        }))
        .await
        .expect("cf_claim must not return McpError");
    assert!(
        !is_error(&claim_result),
        "cf_claim returned tool error: {}",
        result_text(&claim_result)
    );

    // ── Step 5: cf_adr_submit — architect submits the ADR draft ──────────────

    let submit_result = server
        .cf_adr_submit(Parameters(AdrSubmitParams {
            work_item_id: work_item_id_str.clone(),
            title: "Adopt SQLite as the primary persistence store".to_string(),
            content: "## Context\nWe need a lightweight, embeddable persistence layer.\n\
                      ## Decision\nAdopt SQLite via the `rusqlite` crate.\n\
                      ## Consequences\nZero-dependency setup; single-file DB; \
                      suitable for single-host deployments."
                .to_string(),
        }))
        .await
        .expect("cf_adr_submit must not return McpError");
    assert!(
        !is_error(&submit_result),
        "cf_adr_submit returned tool error: {}",
        result_text(&submit_result)
    );

    // ── Step 6: cf_next_step — kernel schedules the adr_review gate ──────────

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

    let next2_raw = result_text(&next2_result);
    serde_json::from_str::<ReadyStep>(&next2_raw)
        .expect("cf_next_step (2) response must deserialise into a ReadyStep")
}

// ── Scenario ───────────────────────────────────────────────────────────────────
//
// Given:   an initialised project with one Architecture / AdrDrafting work item
//          that has been advanced to PendingReview (ADR draft submitted)
// When:    `cf_next_step` returns the step for the adr_review gate
// Then:    if the step routes to a Codex executor, the model must NOT be "o4-mini"
//          (a Claude executor or any non-o4-mini Codex model satisfies the constraint)
//
// Expected failure: the default routing table hardcodes `codex("o4-mini", …)`
// for `WorkType::AdrReview`, so `executor == Codex { model: CodexModel("o4-mini"), .. }`
// and the assertion fails.
#[tokio::test]
async fn adr_review_gate_does_not_route_to_o4_mini() {
    let dir = TempDir::new().expect("tempdir");
    let server = make_server(&dir);

    let step = advance_to_adr_review_state(&server).await;

    let o4_mini = CodexModel::try_new("o4-mini".to_string())
        .expect("'o4-mini' is a valid non-empty CodexModel identifier");

    // Compute whether the step routes to the problematic model.
    // A Claude executor has no Codex model — the o4-mini constraint applies only
    // when the executor is Codex.  Any non-Codex executor satisfies the constraint.
    let routes_to_o4_mini = if let StepAction::GateReview {
        executor: ExecutorSpec::Codex { model, .. },
        ..
    } = &step.action
    {
        model == &o4_mini
    } else {
        false
    };

    // Single behavioral assertion: the adr_review gate must not route to the
    // Codex model "o4-mini" (rejected by the Codex CLI under a ChatGPT account).
    // Update the default routing table to a supported model (e.g. "gpt-5.5") or
    // enable per-install overrides so operators can substitute a supported model.
    assert!(
        !routes_to_o4_mini,
        "adr_review gate must not route to Codex model \"o4-mini\" \
         (rejected by the Codex CLI under a ChatGPT account); \
         got step action: {:?}",
        step.action
    );
}
