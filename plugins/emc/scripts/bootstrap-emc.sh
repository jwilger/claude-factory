#!/usr/bin/env bash
# bootstrap-emc.sh — install emc and verify runtime dependencies, then start emc mcp stdio
set -euo pipefail

EMC_MIN_VERSION="0.1.0"

check_dep() {
  local cmd="$1" name="$2" setup_hint="$3"
  if ! command -v "${cmd}" &>/dev/null; then
    echo "[emc-bootstrap] WARNING: '${cmd}' not found. ${name} is required for 'emc verify'." >&2
    echo "[emc-bootstrap] See docs/SETUP.md for installation instructions: ${setup_hint}" >&2
    echo "[emc-bootstrap] emc will start but the verification gate will not be available." >&2
    return 1
  fi
  return 0
}

# Install emc if not present or outdated
if ! command -v emc &>/dev/null; then
  echo "[emc-bootstrap] Installing emc..." >&2
  if cargo install emc 2>&2; then
    echo "[emc-bootstrap] emc installed from crates.io." >&2
  else
    echo "[emc-bootstrap] crates.io install failed; trying git source..." >&2
    cargo install --git https://git.johnwilger.com/Slipstream/emc 2>&2 || {
      echo "[emc-bootstrap] ERROR: Failed to install emc. Ensure cargo is available and try again." >&2
      exit 1
    }
    echo "[emc-bootstrap] emc installed from git source." >&2
  fi
fi

# Check verification runtime deps (non-fatal — warn only)
VERIFY_AVAILABLE=true
check_dep "lake" "Lean4/lake" "https://github.com/leanprover/elan" || VERIFY_AVAILABLE=false
check_dep "quint" "Quint" "npm install -g @informalsystems/quint" || VERIFY_AVAILABLE=false

if [[ "${VERIFY_AVAILABLE}" == "true" ]]; then
  echo "[emc-bootstrap] All verification dependencies found. emc verify will be fully functional." >&2
else
  echo "[emc-bootstrap] Event model authoring is available; formal verification requires lake + quint." >&2
fi

exec emc mcp stdio
