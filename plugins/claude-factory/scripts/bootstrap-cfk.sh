#!/usr/bin/env bash
set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# bootstrap-cfk.sh — launch the cfk MCP server, building on demand.
#
# IMPORTANT: stdout is the MCP JSON-RPC channel. Every diagnostic, every byte of
# build output MUST go to stderr (>&2), or it corrupts the protocol stream.
#
# Binary resolution order:
#   1. Dev repo (kernel/ sources present): build into .bin/cfk if missing or the
#      kernel sources changed (tracked by a source hash). This is the fast dev
#      loop — edit kernel, restart the MCP server, get a fresh build. No manual
#      cargo build / copy / commit / re-cache dance.
#   2. Prebuilt fallback: a committed bin/cfk (for consumer installs that ship a
#      binary without the kernel sources).
#
# The .bin/ cache is gitignored; bin/ (if present) is the distributed fallback.
# ─────────────────────────────────────────────────────────────────────────────

PLUGIN_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO_ROOT="$(cd "${PLUGIN_ROOT}/../.." && pwd)"
KERNEL_DIR="${REPO_ROOT}/kernel"

CACHE_DIR="${PLUGIN_ROOT}/.bin"
CACHED_BIN="${CACHE_DIR}/cfk"
HASH_FILE="${CACHE_DIR}/.kernel_src_hash"
PREBUILT_BIN="${PLUGIN_ROOT}/bin/cfk"

log() { echo "[claude-factory] $*" >&2; }

# Compute a stable hash over the kernel sources that affect the built binary.
kernel_src_hash() {
  find "${KERNEL_DIR}" \
    -type f \( -name '*.rs' -o -name 'Cargo.toml' -o -name 'Cargo.lock' \) \
    -not -path '*/target/*' -print0 \
    | sort -z \
    | xargs -0 sha256sum \
    | sha256sum \
    | awk '{print $1}'
}

# Resolve a cargo invocation: prefer cargo on PATH, else fall back to the Nix
# devshell so a fresh checkout builds without manual toolchain setup.
cargo_cmd() {
  if command -v cargo >/dev/null 2>&1; then
    echo "cargo"
  elif command -v nix >/dev/null 2>&1 && [[ -f "${REPO_ROOT}/flake.nix" ]]; then
    echo "nix develop ${REPO_ROOT} --command cargo"
  else
    return 1
  fi
}

# Build into the cache if missing or stale. Returns non-zero on any failure.
# NOTE: this function is invoked in a conditional, which disables `set -e`
# inside it, so every fallible step is checked explicitly.
build_if_stale() {
  local want have built
  want="$(kernel_src_hash)"
  have="$(cat "${HASH_FILE}" 2>/dev/null || true)"

  if [[ -x "${CACHED_BIN}" && "${want}" == "${have}" ]]; then
    return 0  # up to date
  fi

  local cargo
  if ! cargo="$(cargo_cmd)"; then
    log "ERROR: kernel sources changed but neither cargo nor nix is available to build."
    log "Install the Rust toolchain (or run inside 'nix develop') and retry."
    return 1
  fi

  log "kernel sources changed — building cfk (this can take a minute)…"
  # Build from the kernel workspace root; all cargo output to stderr so it
  # never pollutes the MCP stdout channel.
  if ! ( cd "${KERNEL_DIR}" && ${cargo} build --release --bin cfk ) >&2; then
    log "ERROR: cargo build failed."
    return 1
  fi

  # Resolve the actual target directory (honors CARGO_TARGET_DIR and .cargo
  # config); fall back to the in-tree default if metadata is unavailable.
  local target_dir built
  target_dir="$( ( cd "${KERNEL_DIR}" && ${cargo} metadata --format-version 1 --no-deps 2>/dev/null ) \
    | tr ',' '\n' | grep '"target_directory"' | head -1 \
    | sed 's/.*"target_directory":"//; s/".*//' )"
  [[ -n "${target_dir}" ]] || target_dir="${CARGO_TARGET_DIR:-${KERNEL_DIR}/target}"
  built="${target_dir}/release/cfk"
  if [[ ! -x "${built}" ]]; then
    log "ERROR: build reported success but ${built} is missing."
    return 1
  fi

  mkdir -p "${CACHE_DIR}"
  if ! cp "${built}" "${CACHED_BIN}"; then
    log "ERROR: failed to copy built binary into ${CACHED_BIN}."
    return 1
  fi
  chmod +x "${CACHED_BIN}"
  echo "${want}" > "${HASH_FILE}"
  log "built cfk -> ${CACHED_BIN}"
  return 0
}

# ── Resolve which binary to exec ─────────────────────────────────────────────
CFK_BIN=""
if [[ -d "${KERNEL_DIR}" ]]; then
  if build_if_stale; then
    CFK_BIN="${CACHED_BIN}"
  fi
fi

if [[ -z "${CFK_BIN}" || ! -x "${CFK_BIN}" ]]; then
  if [[ -x "${PREBUILT_BIN}" ]]; then
    CFK_BIN="${PREBUILT_BIN}"
  fi
fi

if [[ -z "${CFK_BIN}" || ! -x "${CFK_BIN}" ]]; then
  log "ERROR: no cfk binary available (build failed and no prebuilt ${PREBUILT_BIN})."
  exit 1
fi

# cfk takes the product-repo root as its first positional argument (see
# kernel/crates/cfk-mcp/src/main.rs). Claude Code launches plugin MCP servers
# with the project directory as CWD and exports CLAUDE_PROJECT_DIR; prefer the
# explicit env var and fall back to CWD.
exec "${CFK_BIN}" "${CLAUDE_PROJECT_DIR:-$PWD}"
