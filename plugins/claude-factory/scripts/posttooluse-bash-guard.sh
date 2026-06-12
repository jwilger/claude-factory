#!/usr/bin/env bash
# PostToolUse(Bash) guardrail — closes the Bash bypass.
#
# The PreToolUse guardrail only matches the Write and Edit tools, so a file
# mutation performed through Bash (sed -i, tee, shell redirection, cp/mv) never
# reaches it. This hook runs after every Bash command and surfaces any change to
# protected product source that the editing session is not leased to make,
# reusing the same kernel decision (`cfk guardrail-check`) as the PreToolUse gate.
#
# It cannot un-write a file; it fails the tool (exit 2) so the change is surfaced
# to Claude, which must revert it, obtain a lease, or set LEASE_BYPASS. The block
# self-clears once the offending change is reverted or becomes authorized.

input=$(cat)

cfk="${CLAUDE_PLUGIN_ROOT}/bin/cfk"
root="${CLAUDE_PROJECT_DIR:-$PWD}"

[ -x "$cfk" ] || exit 0
command -v git >/dev/null 2>&1 || exit 0
git -C "$root" rev-parse --is-inside-work-tree >/dev/null 2>&1 || exit 0

session=$(printf '%s' "$input" | python3 -c '
import sys, json
try:
    print(json.load(sys.stdin).get("session_id", ""))
except Exception:
    pass
' 2>/dev/null)

# Changed paths in the working tree (porcelain: strip the 2-char status + space;
# collapse "old -> new" renames to the new path).
mapfile -t changed < <(
  git -C "$root" -c core.quotepath=false status --porcelain \
    | sed -e 's/^...//' -e 's/.* -> //'
)

blocked=()
for f in "${changed[@]}"; do
  [ -n "$f" ] || continue
  [ -e "$root/$f" ] || continue
  if ! "$cfk" guardrail-check "$root" "$root/$f" "$session" >/dev/null 2>&1; then
    blocked+=("$f")
  fi
done

if [ "${#blocked[@]}" -gt 0 ]; then
  {
    echo "Guardrail: protected product source was modified outside the factory TDD workflow"
    echo "(e.g. via Bash), with no active lease for this session:"
    printf '  - %s\n' "${blocked[@]}"
    echo "Revert these changes, obtain a lease via /claude-factory:develop, or set"
    echo ".claude-factory/LEASE_BYPASS to override the guardrail for this project."
  } >&2
  exit 2
fi

exit 0
