#!/usr/bin/env bash
# SessionStart hook for Claude-Factory.
#
# SessionStart only supports `command` hooks (not `prompt` hooks), so this
# script emits orienting context to stdout. Whatever it prints is injected
# into the session as additional context for the agent.
#
# Two thin, idempotent responsibilities, both gated on this being a factory repo:
#   1. Ensure the event-staging pre-commit hook is installed (non-clobbering),
#      so managed repos enforce that the append-only event log is committed.
#   2. Emit orienting context asking the agent to surface the status dashboard.
set -euo pipefail

project_dir="${CLAUDE_PROJECT_DIR:-$PWD}"

# Not a factory project → say nothing, do nothing.
[ -d "${project_dir}/.claude-factory" ] || exit 0

# ── 1. Ensure the event-staging pre-commit hook (non-clobbering) ──────────────
installed_note=""
if command -v git >/dev/null 2>&1 \
  && git -C "${project_dir}" rev-parse --is-inside-work-tree >/dev/null 2>&1; then

  # Effective hooks dir: honor core.hooksPath, else the repo's default.
  hooks_dir="$(git -C "${project_dir}" config --get core.hooksPath 2>/dev/null || true)"
  [ -n "${hooks_dir}" ] || hooks_dir="$(git -C "${project_dir}" rev-parse --git-path hooks 2>/dev/null || true)"
  case "${hooks_dir}" in
    "") : ;;          # could not determine — skip
    /*) : ;;          # absolute
    *)  hooks_dir="${project_dir}/${hooks_dir}" ;;
  esac

  precommit="${hooks_dir%/}/pre-commit"
  # Only create when absent — never clobber an existing hook.
  if [ -n "${hooks_dir}" ] && [ ! -e "${precommit}" ]; then
    mkdir -p "${hooks_dir}"
    cat > "${precommit}" <<'HOOK'
#!/usr/bin/env bash
# Claude-Factory: the .claude-factory/events/ log is append-only and the source
# of truth for project state. Refuse to commit while any event file is unstaged,
# so committed history never silently desyncs from the running state.
set -euo pipefail
UNSTAGED=$(
  {
    git ls-files --others --exclude-standard -- ':(top).claude-factory/events/'
    git diff --name-only -- ':(top).claude-factory/events/'
  } | sort -u
)
if [ -n "${UNSTAGED}" ]; then
  echo "[claude-factory] ERROR: factory event-log files are not staged for commit:" >&2
  echo "${UNSTAGED}" | sed 's/^/  - /' >&2
  echo "[claude-factory] The event log must be committed alongside your changes." >&2
  echo "[claude-factory] Stage them:  git add .claude-factory/events" >&2
  exit 1
fi
HOOK
    chmod +x "${precommit}"
    installed_note=" (installed the event-staging pre-commit hook at ${precommit})"
  fi
fi

# ── 2. Orienting context ──────────────────────────────────────────────────────
cat <<EOF
A Claude-Factory project is initialized in this directory (.claude-factory/ exists).${installed_note}
Call the \`cf_status\` MCP tool and display a brief factory status summary to orient the user.
EOF

exit 0
