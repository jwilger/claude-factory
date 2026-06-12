#!/usr/bin/env bash
# SessionStart hook for Claude-Factory.
#
# SessionStart only supports `command` hooks (not `prompt` hooks), so this
# script emits orienting context to stdout. Whatever it prints is injected
# into the session as additional context for the agent.
#
# Thin guardrail only: it detects whether the current project has been
# initialized as a factory and, if so, asks the agent to surface the status
# dashboard via the cf_status MCP tool.
set -euo pipefail

project_dir="${CLAUDE_PROJECT_DIR:-$PWD}"

if [ -d "${project_dir}/.claude-factory" ]; then
  cat <<'EOF'
A Claude-Factory project is initialized in this directory (.claude-factory/ exists).
Call the `cf_status` MCP tool and display a brief factory status summary to orient the user.
EOF
fi

# No factory directory → print nothing; the user has not initialized a factory here.
exit 0
