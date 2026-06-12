#!/usr/bin/env bash
set -euo pipefail

PLUGIN_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CFW_BIN="${PLUGIN_ROOT}/bin/cfk"

if [[ ! -x "${CFW_BIN}" ]]; then
  echo "[claude-factory] ERROR: cfk binary not found at ${CFW_BIN}." >&2
  echo "[claude-factory] Build it with: cargo build --release --bin cfk" >&2
  echo "[claude-factory] Then copy: cp kernel/target/release/cfk plugins/claude-factory/bin/cfk" >&2
  exit 1
fi

# cfk takes the product-repo root as its first positional argument (see
# kernel/crates/cfk-mcp/src/main.rs). Claude Code launches plugin MCP servers
# with the project directory as CWD and exports CLAUDE_PROJECT_DIR; prefer the
# explicit env var and fall back to CWD.
exec "${CFW_BIN}" "${CLAUDE_PROJECT_DIR:-$PWD}"
