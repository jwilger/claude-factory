# Automated per-slice promotion chain with conditional triage gates

**Date:** 2026-06-16
**Status:** Accepted (2026-06-16)

---

## Context

Claude-Factory's six phases (discovery → event modeling → architecture → design
system → development → review) are each modeled as an independent backlog of work
items. The conductor (`cf_next_step`) only automates **two** of the five
inter-phase transitions:

- **Discovery → EventModeling** — `cf_discovery_approve` spawns one
  `EventModelAuthoring` item per workflow.
- **Development → Review** — a slice reaching `TddPhase::Done` auto-completes and
  spawns a paired `PrCommentTriage` review item.

The other three transitions (EventModeling → Architecture, Architecture →
DesignSystem, DesignSystem → Development) have **no automation**. As observed
during dogfooding (2026-06-15), this stranded 22 formally-verified event-model
slices in the modeling phase with nothing downstream: completing a modeling item
created zero architecture or development work. The only bridge,
`cf_ingest_slices`, was never invoked by the conductor, and the
`DesignCrossCheckCompleted` event's `generated_item_ids` were discarded by an
empty handler. The factory therefore could not carry a feature end-to-end
without manual `cf_backlog_add` at every phase boundary.

A second issue is **granularity mismatch**: EventModeling work items are
per-*workflow*, but the verified emc model decomposes each workflow into many
finer-grained *slices* (`read_verified_slices`). Development, the existing
`cf_ingest_slices` target, and the Development → Review automation all operate at
the *slice* level.

## Decision

**The conductor automatically promotes each formally-verified emc slice through
the full pipeline. The promotion unit is the slice; architecture and design are
per-slice *triage gates* that fast-pass by default and only spawn real work when
the slice demands it.**

### The chain

When a workflow's event model is verified (`WorkflowReadinessDeclared`), for each
of its slices the kernel spawns an `ArchitectureTriage` work item carrying the
slice's `emc_slug`. Each slice then flows independently:

```
verified slice
  → ArchitectureTriage  (agent, interactive)
       needs decision → AdrDrafting → AdrReview (loop) → accept → advance
       fast-pass       → advance
  → DesignTriage
       kernel pre-check: does the slice touch UI? (views/read-models/screens)
         no UI  → fast-pass → advance
         has UI → agent (interactive): is the needed atomic-design inventory present?
                    gaps → DesignSystemBuild (interactive) → advance
                    complete → advance
  → Development  (existing TDD slice machinery)
  → Review       (existing Development → Review automation)
```

### Triage semantics

- **Fast-pass is the default.** A triage gate that finds no new architectural
  decision (or no UI surface) completes immediately and advances the slice. Only
  genuine gaps spawn `AdrDrafting`/`AdrReview` or `DesignSystemBuild` work.
- **Architecture triage is judgment** (an agent step): "does this slice require a
  new or changed cross-cutting decision?" In practice a project's first slices
  establish the architectural baseline (several ADRs); later slices mostly
  fast-pass.
- **Design triage has a deterministic pre-check** — whether a slice touches the
  UI at all is derivable from the event model, so pure command/automation slices
  fast-pass with no agent call. The remaining judgment (is the design inventory
  sufficient; build the gaps) is interactive. Component ownership and the design
  inventory are governed by ADR 0012.
- Triage gates that surface decisions for a human follow the existing non-dev
  gate semantics (reviewers raise findings; the human decides).

### Mechanism

- New `WorkType`s `ArchitectureTriage` and `DesignTriage` (existing `AdrDrafting`,
  `AdrReview`, `DesignSystemBuild`, development and review types are reused). Both
  triage types route to Claude in the kernel routing table (ADR 0005).
- Promotion is implemented as scan-and-promote blocks in `cf_next_step`, mirroring
  the two existing automated transitions.
- `cf_ingest_slices`' slice-reading core is repointed to emit chain-heads
  (`ArchitectureTriage`) rather than Development items, and is fired automatically
  on verification; it remains available as a manual backfill/fallback tool. Slice
  ingestion stays idempotent on `emc_slug`.

## Consequences

- The factory can carry a feature from a verified model to a merged PR with no
  manual `cf_backlog_add`. The 22 stranded slices can be backfilled by replaying
  them through the chain.
- Architecture and the design system are built **just-in-time, per slice** — early
  slices carry the baseline work, later slices fast-pass. This is intended, not a
  side effect.
- Per-slice triage adds two lightweight gates to every slice. The cost is bounded
  by the deterministic design pre-check and the fast-pass default; the veto/pass
  ratio should be measured during M7 and revisited if triage proves noisy.
- The conductor remains a dumb dispatcher: all promotion logic is deterministic
  kernel state-machine code (ADR 0001/0002); agents only produce artifacts the
  kernel validates.
- Until the factory can run its own architecture phase (M9), these ADRs are
  authored directly rather than through the factory loop.
