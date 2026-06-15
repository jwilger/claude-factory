//! cfk — Claude-Factory kernel.
//!
//! Two entry points share one binary:
//!
//! * `cfk [project-root]` — run the rmcp stdio MCP server, exposing `cf_*` tools
//!   to the conductor session. Started by
//!   `plugins/claude-factory/scripts/bootstrap-cfk.sh`, which passes the
//!   product-repo root as the first positional argument. If omitted, the current
//!   working directory is used.
//!
//! * `cfk guardrail-check <project-root> <file> <session>` — evaluate the
//!   product-source edit guardrail for a single edit and exit 0 (allow) or 2
//!   (block). Invoked by the `PreToolUse` hook. Any evaluation error fails
//!   closed (exit 2).

#![expect(
    clippy::print_stderr,
    clippy::exit,
    reason = "this is the cfk CLI binary entry point; the guardrail-check subcommand reports its verdict to the PreToolUse hook via the process exit code (0 allow / 2 block) and a human-readable stderr message"
)]

use std::path::Path;

use anyhow::Context;
use cfk_core::types::lease::SessionIdentity;
use cfk_engine::guardrail::check_guardrail;
use cfk_mcp::server::CfkServer;
use chrono::Utc;
use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args();
    let _bin = args.next();
    let first = args.next();

    if first.as_deref() == Some("guardrail-check") {
        // Synchronous, short-lived path — exits the process directly.
        run_guardrail_check(&args.collect::<Vec<_>>());
    }

    run_server(first).await
}

/// Run the MCP stdio server. `project_root_arg` is the first CLI argument, if any.
async fn run_server(project_root_arg: Option<String>) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let project_root = project_root_arg.map_or_else(
        || std::env::current_dir().context("current directory must be accessible"),
        |s| Ok(std::path::PathBuf::from(s)),
    )?;

    tracing::info!("cfk starting, project_root={}", project_root.display());

    let server = CfkServer::load(project_root).await.context("failed to load project state")?;

    let service = server
        .serve(rmcp::transport::stdio())
        .await
        .context("MCP handshake failed")?;

    service
        .waiting()
        .await
        .context("MCP server exited with error")?;

    Ok(())
}

/// Evaluate the edit guardrail and exit the process: 0 = allow, 2 = block.
///
/// Expects positional args `<project-root> <file> <session>`. Any malformed
/// input or evaluation error fails closed (block), so a broken guardrail never
/// silently permits an unguarded edit.
fn run_guardrail_check(args: &[String]) -> ! {
    let [project_root, file, session] = args else {
        eprintln!("usage: cfk guardrail-check <project-root> <file> <session>");
        std::process::exit(2);
    };

    let Ok(session_identity) = SessionIdentity::try_new(session.clone()) else {
        eprintln!("guardrail: empty session identity; blocking (fail closed)");
        std::process::exit(2);
    };

    match check_guardrail(Path::new(project_root), Path::new(file), &session_identity, Utc::now()) {
        Ok(decision) if decision.is_allowed() => std::process::exit(0),
        Ok(_) => {
            eprintln!(
                "Blocked: editing product source requires an active cfk lease for this session. \
                 Start the factory's TDD workflow (/claude-factory:develop), or set \
                 .claude-factory/LEASE_BYPASS to temporarily override the guardrail for this project."
            );
            std::process::exit(2);
        }
        Err(error) => {
            eprintln!("guardrail evaluation failed; blocking (fail closed): {error}");
            std::process::exit(2);
        }
    }
}
