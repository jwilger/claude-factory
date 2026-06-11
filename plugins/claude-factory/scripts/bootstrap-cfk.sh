#!/usr/bin/env bash
set -euo pipefail

PLUGIN_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KERNEL_ROOT="$(cd "${PLUGIN_ROOT}/../../kernel" && pwd)"
BIN_DIR="${PLUGIN_ROOT}/.bin"
CFW_BIN="${BIN_DIR}/cfk"

mkdir -p "${BIN_DIR}"

# Check if cfk binary is already built and up to date
KERNEL_HASH_FILE="${BIN_DIR}/.kernel_src_hash"
CURRENT_HASH=""
if command -v find &>/dev/null && command -v sha256sum &>/dev/null; then
  CURRENT_HASH=$(find "${KERNEL_ROOT}" -name "*.rs" -o -name "Cargo.toml" -o -name "Cargo.lock" 2>/dev/null \
    | sort | xargs sha256sum 2>/dev/null | sha256sum | cut -d' ' -f1 || echo "")
fi

CACHED_HASH=""
if [[ -f "${KERNEL_HASH_FILE}" ]]; then
  CACHED_HASH=$(cat "${KERNEL_HASH_FILE}")
fi

if [[ -x "${CFW_BIN}" && -n "${CURRENT_HASH}" && "${CURRENT_HASH}" == "${CACHED_HASH}" ]]; then
  exec "${CFW_BIN}" mcp stdio
fi

# Need to build
echo "[claude-factory] Building cfk kernel..." >&2

if ! command -v cargo &>/dev/null; then
  echo "[claude-factory] ERROR: cargo not found. Run 'nix develop' or see docs/SETUP.md for toolchain setup." >&2
  exit 1
fi

(cd "${KERNEL_ROOT}" && cargo build --release --bin cfk 2>&2)

cp "${KERNEL_ROOT}/target/release/cfk" "${CFW_BIN}"
chmod +x "${CFW_BIN}"

if [[ -n "${CURRENT_HASH}" ]]; then
  echo "${CURRENT_HASH}" > "${KERNEL_HASH_FILE}"
fi

echo "[claude-factory] cfk built successfully." >&2
exec "${CFW_BIN}" mcp stdio
