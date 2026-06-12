#!/usr/bin/env bash
# Claude-Factory statusline script.
#
# Reads .claude-factory/events/v1/*.json from the current working directory
# (passed as cwd in the JSON stdin context) and prints a one-line factory
# dashboard. Outputs nothing if this is not a factory-initialized repo.
#
# Output format: [CF] Dev:N Rev:N Disc:N Arch:N Des:N | blocked:N
#
# Designed to be lightweight — reads files directly without starting cfk.

set -euo pipefail

# Read stdin JSON to get cwd; fall back to $PWD if parsing fails.
CWD="$PWD"
if command -v python3 &>/dev/null; then
    PARSED=$(python3 -c "
import json, sys
try:
    data = json.load(sys.stdin)
    print(data.get('cwd', ''))
except Exception:
    print('')
" 2>/dev/null) || true
    [[ -n "$PARSED" ]] && CWD="$PARSED"
fi

EVENT_DIR="$CWD/.claude-factory/events/v1"
[[ -d "$EVENT_DIR" ]] || exit 0

# Count work items by phase and status from the event log.
python3 - "$EVENT_DIR" <<'PYEOF'
import json, os, sys, glob

event_dir = sys.argv[1]
files = sorted(glob.glob(os.path.join(event_dir, "*.json")))

# Replay: track work items.
items = {}  # id -> {phase, status}

for path in files:
    try:
        env = json.loads(open(path).read())
    except Exception:
        continue
    payload = env.get("payload", {})
    kind = payload.get("type") or list(payload.keys())[0] if payload else None

    # Detect by known keys since events are tagged structs.
    if "WorkItemAdded" in payload:
        wi = payload["WorkItemAdded"]["work_item"]
        items[wi["id"]] = {"phase": wi.get("phase", "?"), "status": "Ready"}
    elif "LeaseGranted" in payload:
        wid = payload["LeaseGranted"]["lease"]["work_item_id"]
        if wid in items:
            items[wid]["status"] = "InProgress"
    elif "LeaseReleased" in payload:
        wid = payload["LeaseReleased"]["work_item_id"]
        if wid in items:
            items[wid]["status"] = "Ready"
    elif "WorkItemCompleted" in payload:
        wid = payload["WorkItemCompleted"]["work_item_id"]
        if wid in items:
            items[wid]["status"] = "Done"
    elif "WorkItemAbandoned" in payload:
        wid = payload["WorkItemAbandoned"]["work_item_id"]
        if wid in items:
            items[wid]["status"] = "Abandoned"

# Count by phase for active (not Done/Abandoned) items.
phase_counts = {}
active_total = 0
for item in items.values():
    if item["status"] in ("Done", "Abandoned"):
        continue
    phase = item["phase"]
    phase_counts[phase] = phase_counts.get(phase, 0) + 1
    active_total += 1

if active_total == 0 and not items:
    # No work items at all — freshly initialized.
    print("[CF] initialized")
    sys.exit(0)

# Map phase kinds to short labels.
labels = {
    "Development": "Dev",
    "Review": "Rev",
    "Discovery": "Disc",
    "Architecture": "Arch",
    "Design": "Des",
    "EventModeling": "EMC",
}

parts = []
for phase_key, label in labels.items():
    count = phase_counts.get(phase_key, 0)
    if count > 0:
        parts.append(f"{label}:{count}")

if parts:
    print("[CF] " + " ".join(parts) + f" | {active_total} active")
elif active_total == 0:
    print("[CF] idle")
else:
    print(f"[CF] {active_total} active")
PYEOF
