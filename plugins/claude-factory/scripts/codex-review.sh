#!/usr/bin/env bash
# codex-review.sh — programmatic cross-family (GPT) review of a branch diff.
#
# Wraps `codex review --base <ref>` for headless use by the factory's review
# phase: it reviews the current branch against <ref> with codex (gpt-5.5) and
# prints the reviewer's findings (severity-tagged `[P#] title — file:line`) to
# stdout. codex's verbose telemetry is sent to stderr (quieted with RUST_LOG).
#
# Usage: codex-review.sh [base-ref]   (base-ref defaults to "main")
#
# Exit codes:
#   0  review ran; findings (if any) are on stdout
#   1  codex CLI not found
#   2  codex review failed to run
#
# Pair with an independent Claude (Opus) review for the multi-front, multi-agent
# PR gate — the two model families surface complementary defect classes.

set -euo pipefail

BASE_REF="${1:-main}"

if ! command -v codex >/dev/null 2>&1; then
  echo "[codex-review] ERROR: codex CLI not found. Install via npm install -g @openai/codex." >&2
  exit 1
fi

# codex review streams findings to stdout; RUST_LOG=error suppresses its INFO/otel
# log spew (which otherwise floods stderr). The diff is computed by codex itself
# from `--base`, so run it from the product repo's working tree.
RUST_LOG="${RUST_LOG:-error}" codex review --base "${BASE_REF}" || {
  echo "[codex-review] ERROR: 'codex review --base ${BASE_REF}' failed." >&2
  exit 2
}
