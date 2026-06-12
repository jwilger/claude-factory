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

exec "${CFW_BIN}" mcp stdio
