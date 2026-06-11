#!/usr/bin/env bash
# codex-runner.sh — invoke codex exec with structured output for kernel-routed GPT steps
#
# Usage: codex-runner.sh <model> <effort> <schema-file> <prompt-file> <output-file>
#
# - model:       e.g. "o3", "o4-mini"
# - effort:      e.g. "high", "medium", "low"  (maps to codex reasoning effort)
# - schema-file: path to JSON Schema file for structured output
# - prompt-file: path to plain-text prompt file
# - output-file: path where the final structured JSON will be written
#
# Exit codes:
#   0  success; output-file contains valid JSON matching schema
#   1  codex exec failed
#   2  output-file missing or invalid after run

set -euo pipefail

MODEL="${1:?model argument required}"
EFFORT="${2:?effort argument required}"
SCHEMA_FILE="${3:?schema-file argument required}"
PROMPT_FILE="${4:?prompt-file argument required}"
OUTPUT_FILE="${5:?output-file argument required}"

if ! command -v codex &>/dev/null; then
  echo "[codex-runner] ERROR: codex CLI not found. Install via npm install -g @openai/codex." >&2
  exit 1
fi

CODEX_VERSION=$(codex --version 2>/dev/null | head -1 || echo "unknown")
REQUIRED_MIN="0.100.0"
echo "[codex-runner] codex version: ${CODEX_VERSION}" >&2

PROMPT_TEXT=$(cat "${PROMPT_FILE}")

codex exec \
  -m "${MODEL}" \
  -c "reasoning_effort=${EFFORT}" \
  --output-schema "${SCHEMA_FILE}" \
  -o "${OUTPUT_FILE}" \
  --sandbox workspace-write \
  --json \
  "${PROMPT_TEXT}" \
  1>&2

if [[ ! -f "${OUTPUT_FILE}" ]]; then
  echo "[codex-runner] ERROR: output file not created at ${OUTPUT_FILE}" >&2
  exit 2
fi

# Validate JSON
if ! python3 -c "import json,sys; json.load(open('${OUTPUT_FILE}'))" 2>/dev/null && \
   ! node -e "JSON.parse(require('fs').readFileSync('${OUTPUT_FILE}','utf8'))" 2>/dev/null; then
  echo "[codex-runner] ERROR: output file is not valid JSON" >&2
  exit 2
fi

echo "[codex-runner] success: output written to ${OUTPUT_FILE}" >&2
exit 0
