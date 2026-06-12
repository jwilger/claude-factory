#!/usr/bin/env bash
# PreToolUse guardrail for Write/Edit.
#
# Thin shell: all policy lives in the cfk kernel. This script extracts the
# edited file path and the Claude session id from the hook's stdin JSON and
# delegates the decision to `cfk guardrail-check`:
#   exit 0 → allow the edit
#   exit 2 → block the edit (cfk's stderr explains why; Claude Code surfaces it)
#
# Failure to parse the hook input fails OPEN (allow): the kernel-side check is
# the authoritative enforcement layer, and a parsing glitch must not brick all
# editing. The guardrail only governs projects that contain a .claude-factory/
# directory; cfk allows everything else.

input=$(cat)

cfk="${CLAUDE_PLUGIN_ROOT}/bin/cfk"
root="${CLAUDE_PROJECT_DIR:-$PWD}"

# Extract tool_input.file_path and session_id, one per line (handles spaces).
mapfile -t fields < <(printf '%s' "$input" | python3 -c '
import sys, json
try:
    d = json.load(sys.stdin)
except Exception:
    sys.exit(0)
print(d.get("tool_input", {}).get("file_path", ""))
print(d.get("session_id", ""))
' 2>/dev/null)

file="${fields[0]:-}"
session="${fields[1]:-}"

# No file path (not a file-writing tool, or parse failed) → nothing to guard.
[ -z "$file" ] && exit 0

# No cfk binary → cannot evaluate; fail open rather than block all edits.
[ -x "$cfk" ] || exit 0

if "$cfk" guardrail-check "$root" "$file" "$session"; then
  exit 0
else
  # cfk printed the block reason to stderr; exit 2 tells Claude Code to block.
  exit 2
fi
