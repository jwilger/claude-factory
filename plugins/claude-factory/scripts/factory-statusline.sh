#!/usr/bin/env bash
# Claude-Factory statusline script.
#
# Reads .claude-factory/eventstore/events/*.jsonl (eventcore-fs) from the
# current working directory (passed as cwd in the JSON stdin context) and
# prints a one-line factory dashboard. Outputs nothing if this is not a
# factory-initialized repo.
#
# Output format: [CF] Dev:N Rev:N Disc:N Arch:N Des:N | N active
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

EVENT_DIR="$CWD/.claude-factory/eventstore/events"
[[ -d "$EVENT_DIR" ]] || exit 0

# Count work items by phase and status from the event log.
# Phase values are serde snake_case from PhaseKind: discovery, event_modeling,
# architecture, design_system, development, review.
python3 - "$EVENT_DIR" <<'PYEOF'
import json, os, sys, glob

event_dir = sys.argv[1]
files = sorted(glob.glob(os.path.join(event_dir, "*.jsonl")))

# Replay: track work items from eventcore-fs JSONL format.
# Each .jsonl file is a transaction with one "header" line and one or more
# "event" lines. Factory events live in the "event_data" field of event records.
items = {}  # id -> {phase, status}

for path in files:
    try:
        for line in open(path):
            line = line.strip()
            if not line:
                continue
            obj = json.loads(line)
            if obj.get("record") != "event":
                continue
            payload = obj.get("event_data", {})
            ptype = payload.get("type", "")

            if ptype == "work_item_added":
                wi = payload["work_item"]
                items[wi["id"]] = {"phase": wi.get("phase", "?"), "status": wi.get("status", "ready")}
            elif ptype == "lease_granted":
                wid = payload["lease"]["work_item_id"]
                if wid in items:
                    items[wid]["status"] = "in_progress"
            elif ptype == "lease_released":
                wid = payload["work_item_id"]
                if wid in items:
                    items[wid]["status"] = "ready"
            elif ptype == "work_item_completed":
                wid = payload["work_item_id"]
                if wid in items:
                    items[wid]["status"] = "done"
            elif ptype == "work_item_abandoned":
                wid = payload["work_item_id"]
                if wid in items:
                    items[wid]["status"] = "abandoned"
    except Exception:
        continue

# Count by phase for active (not done/abandoned) items.
phase_counts = {}
active_total = 0
for item in items.values():
    if item["status"] in ("done", "abandoned"):
        continue
    phase = item["phase"]
    phase_counts[phase] = phase_counts.get(phase, 0) + 1
    active_total += 1

if not items:
    # Initialized but no work items yet.
    print("[CF] initialized")
    sys.exit(0)

# Map serde snake_case phase names to short display labels.
labels = {
    "development": "Dev",
    "review": "Rev",
    "discovery": "Disc",
    "architecture": "Arch",
    "design_system": "Des",
    "event_modeling": "EMC",
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
