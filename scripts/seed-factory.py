#!/usr/bin/env python3
"""
Seed the Claude-Factory event log for this repo.

Initializes .claude-factory/events/v1/ with:
  1. ProjectInitialized
  2. Initial backlog — the factory's own next features, seeded as work items
     so they flow through the factory's own process.

Run once from the repo root:
    python3 scripts/seed-factory.py

Idempotent: exits without writing if the event log already exists.
"""

import json
import os
import sys
import uuid
from datetime import datetime, timezone

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EVENT_DIR = os.path.join(ROOT, ".claude-factory", "events", "v1")


def now_iso() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3] + "Z"


def write_event(seq: int, payload: dict) -> None:
    event_id = str(uuid.uuid4())
    envelope = {
        "id": event_id,
        "sequence": seq,
        "timestamp": now_iso(),
        "payload": payload,
    }
    filename = f"{seq:010d}-{event_id}.json"
    path = os.path.join(EVENT_DIR, filename)
    with open(path, "w") as f:
        json.dump(envelope, f, indent=2)
    print(f"  [{seq:03d}] {payload['type']} → {filename}")


def new_work_item(phase: str, work_type: str, description: str) -> dict:
    return {
        "id": str(uuid.uuid4()),
        "phase": phase,
        "work_type": work_type,
        "status": "ready",
        "description": description,
        "active_lease": None,
        "active_step": None,
    }


def main() -> None:
    if os.path.isdir(EVENT_DIR) and os.listdir(EVENT_DIR):
        print("Event log already exists — nothing to do.")
        print(f"  {EVENT_DIR}")
        sys.exit(0)

    os.makedirs(EVENT_DIR, exist_ok=True)
    print(f"Seeding factory event log at {EVENT_DIR}")

    project_id = str(uuid.uuid4())
    seq = 1

    # ── Event 1: ProjectInitialized ─────────────────────────────────────────
    write_event(seq, {"type": "project_initialized", "id": project_id})
    seq += 1

    # ── Events 2+: Initial backlog ───────────────────────────────────────────
    # These represent the factory's own next planned features, seeded so they
    # flow through the factory's own discovery → development → review process.

    # Discovery: define the snapshot optimization story.
    write_event(seq, {
        "type": "work_item_added",
        "work_item": new_work_item(
            "discovery",
            "socratic_discovery",
            "Snapshot optimization: large event logs replay slowly. Define what "
            "a snapshot format looks like for cfk, when to create snapshots, and "
            "what the migration story is for existing logs.",
        ),
    })
    seq += 1

    # Architecture: ADR for the snapshot format, once discovery is approved.
    write_event(seq, {
        "type": "work_item_added",
        "work_item": new_work_item(
            "architecture",
            "adr_drafting",
            "ADR: event log snapshot format — on-disk layout, triggering policy "
            "(event count threshold), replay strategy (snapshot + tail), and "
            "backwards compatibility with snapshot-free logs.",
        ),
    })
    seq += 1

    # Development: a concrete, small slice to prove the factory builds itself.
    # cf_status currently shows raw counts; a formatted, human-friendly output
    # is the smallest useful kernel feature to implement first.
    write_event(seq, {
        "type": "work_item_added",
        "work_item": new_work_item(
            "development",
            "outer_behavioral_test_writing",
            "cf_status: structured output — return a typed StatusSummary "
            "(phase → {ready, in_progress, done, abandoned} counts) instead of "
            "a raw text blob, so the conductor can render a consistent "
            "dashboard regardless of kernel version.",
        ),
    })
    seq += 1

    print(f"\nDone. {seq - 1} events written.")
    print("\nTo inspect the factory state, start cfk and call cf_status.")
    print("To work the backlog: /claude-factory:work (in this repo)")


if __name__ == "__main__":
    main()
