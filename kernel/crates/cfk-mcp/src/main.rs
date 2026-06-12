//! cfk — Claude-Factory kernel MCP server.
//!
//! Runs as an rmcp stdio server, exposing `cf_*` tools to the conductor
//! session. The server is started by `plugins/claude-factory/scripts/bootstrap-cfk.sh`
//! which passes the product-repo root as the first positional argument.
//!
//! Usage: `cfk [project-root]`
//!
//! If `project-root` is omitted, the current working directory is used.

use anyhow::Context;
use cfk_mcp::server::CfkServer;
use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let project_root = std::env::args().nth(1).map_or_else(
        || std::env::current_dir().context("current directory must be accessible"),
        |s| Ok(std::path::PathBuf::from(s)),
    )?;

    tracing::info!("cfk starting, project_root={}", project_root.display());

    let server = CfkServer::load(project_root).context("failed to load project state")?;

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
