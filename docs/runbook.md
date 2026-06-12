# Claude-Factory Walking Skeleton Runbook

This document records the full traversal of one feature through all phases of
the Claude-Factory process: discovery → event modeling → architecture →
design system → development → review (merge).

It also demonstrates the cycling-phases model: multiple phases can have
in-progress work simultaneously, and the conductor loop dispatches each phase
via the `--phase` filter so multiple Claude Code sessions can operate
independently without stepping on each other.

---

## Prerequisites

1. `cfk` binary built and wired into Claude Code via `.mcp.json`
   (bootstrap script at `plugins/claude-factory/scripts/bootstrap-cfk.sh`).
2. Product repo initialized: `/claude-factory:init` run in the target repo.
3. `emc` installed and wired (for the event-modeling phase).
4. A Gitea (or GitHub) repo for the toy product, accessible via `gh`/`tea`.

---

## Phase 1 — Discovery

**Goal:** produce a validated product brief and a list of workflows to model.

```
/claude-factory:discover
```

The kernel returns a `spawn_agent` step with the `discovery-partner` agent.
The conductor runs the agent in a dialogue loop until the product brief is
drafted. The conductor then presents the brief to you for approval.

**Key tools used (in order):**

| Tool | Purpose |
|---|---|
| `cf_next_step` | Returns discovery `spawn_agent` step |
| `cf_claim` | Acquires lease for the discovery work item |
| Agent (discovery-partner) | Socratic dialogue; produces product brief JSON |
| `cf_discovery_submit` | Submits the brief; kernel transitions to BriefReady |
| `cf_next_step` | Returns `ask_human` step for approval gate |
| `cf_discovery_approve` | Records human approval; kernel queues workflow work items |

**Overlapping WIP:** once discovery is approved, event-modeling work items are
immediately ready. You can open a second Claude Code session and run:

```
/claude-factory:model
```

while a third session handles architecture items:

```
/claude-factory:architect
```

The kernel's `phase_filter` ensures each session only receives steps for its
own phase — no cross-session interference.

---

## Phase 2 — Event Modeling

**Goal:** produce a formally verified event model for each workflow, then have
the kernel ingest the resulting slices into the development backlog.

```
/claude-factory:model
```

**Key tools used:**

| Tool | Purpose |
|---|---|
| `cf_next_step` | Returns modeling `spawn_agent` step |
| `cf_claim` | Acquires lease for the modeling work item |
| Agent (event-modeler) | Drives emc MCP tools; authors the model |
| `cf_submit` | Submits modeling results; kernel ingests slices on verify pass |

The kernel calls `emc verify_project` deterministically. A failed verification
rejects the submission with the emc error — the agent must fix the model before
the kernel will accept it.

---

## Phase 3 — Architecture

**Goal:** propose, review, and accept ADRs; kernel renders `ARCHITECTURE.md`.

```
/claude-factory:architect
```

**Key tools used:**

| Tool | Purpose |
|---|---|
| `cf_next_step` | Returns `spawn_agent` (draft ADR) or `gate_review` (review ADR) |
| `cf_claim` | Acquires lease |
| Agent (architect) | Drafts the ADR |
| `cf_adr_submit` | Submits ADR content; kernel transitions to PendingReview |
| `cf_next_step` | Returns `gate_review` step |
| Agent (adr-reviewer) | Reviews for baseline conflicts; returns verdict |
| `cf_gate` | Records verdict; accepted → kernel renders ARCHITECTURE.md |

Vetoed ADRs cycle back to drafting. The adr-reviewer is intentionally run with
a different perspective than the drafter — cross-family review is built into
the routing defaults.

---

## Phase 4 — Design System

**Goal:** enumerate UI components needed by the verified slices; record them in
the design inventory.

```
/claude-factory:design
```

**Key tools used:**

| Tool | Purpose |
|---|---|
| `cf_next_step` | Returns `spawn_agent` (build component) |
| `cf_claim` | Acquires lease |
| Agent (design-system-builder) | Proposes component per Atomic Design hierarchy |
| `cf_design_add_component` | Records component in inventory |
| `cf_design_cross_check` | Confirms all slices have required components |

The cross-check step is deterministic: the kernel compares slice references
in the verified event model against the inventory and surfaces any gaps.

---

## Phase 5 — Development

**Goal:** build each slice via enforced TDD — red-green-refactor under
independent test-review and implementation-review gates.

```
/claude-factory:develop
```

Development work items are auto-claimed by `cf_next_step` when `session_identity`
is provided. Multiple dev sessions can operate simultaneously on different slices.

**Key tools used (per slice):**

| Tool | Purpose |
|---|---|
| `cf_next_step` | Returns TDD step (WriteTest/Implement/etc.) |
| Agent (test-writer) | Writes outer behavioral test |
| `cf_submit` | Submits test; kernel transitions to TestReviewGate |
| `cf_gate` | Records test review verdict (approve → RedCheck; veto → revise loop) |
| `cf_run_check` | Kernel runs the test suite; parses pass/fail and first error |
| Agent (implementer) | Addresses the first visible error only |
| `cf_run_check` | Kernel runs tests again; tracks failure progression |
| `cf_gate` | Records implementation review verdict |
| `cf_run_check` | Kernel runs linter; strict config |
| `cf_submit` | Slice done |

For drill-down frames (unit tests under a failing integration test), the loop
recurses automatically — the kernel pushes a child TddFrame and the same step
logic applies at the inner scope.

---

## Phase 6 — Review

**Goal:** open a PR, respond to review comments, merge when all-green.

```
/claude-factory:review
```

**Key tools used:**

| Tool | Purpose |
|---|---|
| `cf_next_step` | Returns `open_pr` step |
| `cf_pr_open` | Kernel opens PR via forge adapter |
| `cf_next_step` | Returns `run_pr_poll` |
| `cf_pr_poll` | Kernel polls forge for CI status and new comments |
| `cf_submit` | Submits comment triage; kernel creates triage work items |
| Agent (pr-shepherd-triage) | Addresses each review comment |
| `cf_next_step` | Returns `merge_pr` when all-green |
| `cf_pr_merge` | Kernel merges the PR; slice closed |

---

## Overlapping WIP — the cycling-phases model

The key insight M6 validates: **phases run simultaneously**, not serially.

Once discovery is approved:
- Session A: `/claude-factory:develop --phase development` — builds slices
- Session B: `/claude-factory:architect --phase architecture` — authors ADRs
- Session C: `/claude-factory:design --phase design` — builds components
- Session D: `/claude-factory:review --phase review` — shepherds PRs

Each session calls `cf_next_step` with its `phase_filter`. The kernel's
priority ordering (dev > review > discovery > architecture > design) only
applies when no filter is set — the unscoped `/claude-factory:work` loop
dispatches in priority order across all phases.

The kernel's lease model prevents two sessions from picking up the same work
item. Restart durability means a crashed session can be resumed by simply
re-running the command — the kernel replays events and returns the same step.

---

## Multi-session concurrency — session-per-phase tabs

Running a separate terminal tab per phase is the recommended pattern for
production use. Each tab opens the product repo in Claude Code and runs a
scoped loop:

**Tab 1 — Discovery / overall dispatch**
```
/claude-factory:work
```
Dispatches across all phases in priority order. Use this when you want one
session to drive whatever is most urgent.

**Tab 2 — Development only**
```
/claude-factory:develop
```
Equivalent to `/claude-factory:work --phase development`. Stays in the dev TDD
loop and ignores discovery, architecture, etc.

**Tab 3 — Review only**
```
/claude-factory:review
```
Polls PRs and triages comments. Safe to leave running continuously — the loop
idles when there is nothing to triage.

**Tab 4 — Architecture**
```
/claude-factory:architect
```

**Tab 5 — Design system**
```
/claude-factory:design
```

### Why this is safe

The kernel's lease protocol prevents two sessions from working on the same item:

1. Before executing a non-dev step, the conductor calls `cf_claim`.
2. `cf_claim` reads the current `WorkItemStatus`; if it is `InProgress` (another
   session already holds a lease), it returns an error — the conductor stops and
   reports the conflict rather than double-executing.
3. When a session finishes or crashes, `cf_release` (or a manual call on
   restart) returns the item to `Ready` so any session can pick it up.
4. The entire lease history is event-sourced — replaying events from
   `.claude-factory/events/v1/` always restores the exact lease state.

### Recovering after a crash

If a session crashes mid-step:

1. Open a new Claude Code tab in the same product repo.
2. Run the same phase command (e.g. `/claude-factory:develop`).
3. The kernel replays events; the crashed item is still `InProgress` under
   the old lease.
4. Call `cf_release <work_item_id>` to return it to `Ready`.
5. The conductor loop will immediately pick it up and resume from the last
   completed step.

### PR shepherd as a scheduled routine

The review phase is the lowest-risk candidate for a dark (unattended) loop
because it is entirely driven by forge API state — no code changes, no
human decisions beyond comment triage.

To run the PR shepherd on a schedule (e.g. every 15 minutes):

```bash
# In the product repo, run once to install the cron:
/schedule "Run /claude-factory:review in $(pwd)" --interval 15m
```

The scheduled routine calls `cf_pr_poll` on each active PR work item and
creates triage work items for new comments. You review the triage items in
your next interactive session, or leave a development session open to handle
them continuously.

**Note:** the PR shepherd routine is read-only with respect to code — it only
posts comments and merges when all CI checks and human approvals are recorded.
The kernel enforces the merge gate; the scheduled routine cannot bypass it.

---

## HTTP mode for cfk and emc

Both cfk and emc support an HTTP/bearer-token mode in addition to stdio. HTTP
mode enables a single kernel process to be shared across multiple simultaneous
Claude Code sessions without each session spawning its own process.

**Current recommendation (M8):** stdio mode is sufficient for session-per-phase
tabs because each tab runs its own cfk process and the event log on disk (JSON
files in `.claude-factory/events/v1/`) is the shared-state layer. File system
locks are sufficient at M8 throughput.

**When to consider HTTP mode:**
- More than ~8 simultaneous sessions in the same product repo.
- Cloud-hosted sessions where the product repo is not locally mounted.
- Shared CI environments that need a persistent kernel process.

See `docs/decisions/0008-http-mode-evaluation.md` for the full ADR.

---

## Exit criterion for M6

The walking skeleton is complete when:

1. ✓ Kernel behavioral tests prove overlapping WIP (dev + discovery simultaneous)
   with phase-filter scoping and restart durability (`m6_walking_skeleton` module,
   5 tests, all green).
2. ✓ `design-system-builder` agent exists with Atomic Design methodology.
3. ✓ `/claude-factory:work` command routes phase-specific submissions to the
   correct kernel tools and claims non-dev items before executing.
4. ✓ This runbook documents the full traversal.

## Exit criterion for M8

Concurrency graduation is complete when:

1. ✓ Kernel behavioral tests prove lease contention is handled correctly:
   second claim rejected, release re-enables claim, two sessions on different
   items coexist, next_step idles when all items leased, lease state survives
   restart (`m8_concurrency` module, 5 tests, all green).
2. ✓ This runbook documents session-per-phase tabs, crash recovery, PR shepherd
   scheduling, and the HTTP mode decision.
3. ✓ `docs/decisions/0008-http-mode-evaluation.md` records the stdio-first
   decision and the conditions under which HTTP mode becomes appropriate.
4. ✓ Statusline configuration shows live factory dashboard (active WIP count
   and phase summary) via `cf_status`.
