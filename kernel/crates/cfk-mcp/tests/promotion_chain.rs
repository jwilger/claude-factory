#![expect(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration tests use expect/unwrap for assertion clarity"
)]
//! Behavioural integration tests for the per-slice promotion chain (ADR 0011).
//!
//! A verified emc model on disk must cause `cf_next_step` to spawn the chain-head
//! work item for each slice (Architecture triage), and the chain must advance one
//! phase at a time as items complete. These tests verify durable effects through
//! the public tool surface — no internal plumbing, no mocks.

use std::path::Path;

use cfk_core::types::phase::PhaseKind;
use cfk_engine::forge::MemoryForge;
use cfk_mcp::server::{CfkServer, InitParams, NextStepParams, PhaseFilterParams};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, RawContent};
use tempfile::TempDir;

// ── Helpers ─────────────────────────────────────────────────────────────────

fn result_text(result: &CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| match &c.raw {
            RawContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn result_json(result: &CallToolResult) -> serde_json::Value {
    serde_json::from_str(&result_text(result)).expect("tool result is valid JSON")
}

async fn make_server(dir: &TempDir) -> CfkServer {
    CfkServer::load_with_forge(dir.path().to_path_buf(), MemoryForge::new())
        .await
        .expect("server load")
}

async fn init_project(server: &CfkServer, dir: &TempDir) {
    let result = server
        .cf_init(Parameters(InitParams {
            project_root: Some(dir.path().to_str().unwrap().to_string()),
        }))
        .await
        .expect("cf_init");
    assert!(!result.is_error.unwrap_or(false), "cf_init: {}", result_text(&result));
}

/// Write a formally-verified emc model: a `WorkflowReadinessDeclared` event for
/// the workflow plus a `SliceAdded` event per `(slug, name, kind)`.
fn write_verified_model(root: &Path, workflow: &str, slices: &[(&str, &str, &str)]) {
    let dir = root.join("model").join("events").join("v1");
    std::fs::create_dir_all(&dir).expect("create model events dir");

    let readiness = serde_json::json!({
        "type": "WorkflowReadinessDeclared",
        "payload": { "workflow": workflow },
    });
    std::fs::write(
        dir.join("0000-readiness.json"),
        serde_json::to_string_pretty(&readiness).unwrap(),
    )
    .expect("write readiness");

    for (i, (slug, name, kind)) in slices.iter().enumerate() {
        let ev = serde_json::json!({
            "type": "SliceAdded",
            "payload": {
                "workflow": workflow,
                "slug": slug,
                "name": name,
                "kind": kind,
                "description": format!("{name} description"),
            },
        });
        std::fs::write(
            dir.join(format!("{:04}-slice.json", i + 1)),
            serde_json::to_string_pretty(&ev).unwrap(),
        )
        .expect("write slice");
    }
}

async fn next_step(server: &CfkServer) {
    let result = server
        .cf_next_step(Parameters(NextStepParams {
            phase: None,
            session_identity: None,
        }))
        .await
        .expect("cf_next_step");
    assert!(!result.is_error.unwrap_or(false), "cf_next_step: {}", result_text(&result));
}

async fn backlog(server: &CfkServer, phase: PhaseKind) -> Vec<serde_json::Value> {
    let result = server
        .cf_backlog(Parameters(PhaseFilterParams { phase: Some(phase) }))
        .await
        .expect("cf_backlog");
    result_json(&result)["items"].as_array().cloned().unwrap_or_default()
}

// ── Scenarios ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn verified_slices_spawn_architecture_triage_chain_heads() {
    let dir = TempDir::new().unwrap();
    let server = make_server(&dir).await;
    init_project(&server, &dir).await;
    write_verified_model(
        dir.path(),
        "checkout",
        &[
            ("add-to-cart", "Add item to cart", "command"),
            ("view-cart", "View cart contents", "state_view"),
        ],
    );

    next_step(&server).await;

    let arch = backlog(&server, PhaseKind::Architecture).await;
    assert_eq!(arch.len(), 2, "one architecture triage item per verified slice");
    for item in &arch {
        assert_eq!(item["work_type"], "ArchitectureTriage");
    }
    // Nothing should have jumped ahead to design or development yet.
    assert!(backlog(&server, PhaseKind::DesignSystem).await.is_empty());
    assert!(backlog(&server, PhaseKind::Development).await.is_empty());
}

#[tokio::test]
async fn reconciliation_is_idempotent_across_calls() {
    let dir = TempDir::new().unwrap();
    let server = make_server(&dir).await;
    init_project(&server, &dir).await;
    write_verified_model(dir.path(), "checkout", &[("add-to-cart", "Add to cart", "command")]);

    next_step(&server).await;
    next_step(&server).await;
    next_step(&server).await;

    let arch = backlog(&server, PhaseKind::Architecture).await;
    assert_eq!(arch.len(), 1, "repeated cf_next_step must not re-spawn the chain head");
}

#[tokio::test]
async fn unverified_workflow_slices_are_not_promoted() {
    let dir = TempDir::new().unwrap();
    let server = make_server(&dir).await;
    init_project(&server, &dir).await;

    // SliceAdded with NO WorkflowReadinessDeclared → not verified.
    let mdir = dir.path().join("model").join("events").join("v1");
    std::fs::create_dir_all(&mdir).unwrap();
    let ev = serde_json::json!({
        "type": "SliceAdded",
        "payload": {
            "workflow": "draft", "slug": "wip", "name": "WIP", "kind": "command",
            "description": "unverified",
        },
    });
    std::fs::write(mdir.join("0001-slice.json"), ev.to_string()).unwrap();

    next_step(&server).await;

    assert!(
        backlog(&server, PhaseKind::Architecture).await.is_empty(),
        "slices from unverified workflows must not be promoted"
    );
}
