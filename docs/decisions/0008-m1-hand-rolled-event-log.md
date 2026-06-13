# 0008 — M1 ships a hand-rolled JSON event log; eventcore deferred to M2

**Date:** 2026-06-11  
**Status:** Superseded by ADR 0010 (eventcore 0.9 + eventcore-fs event store)

## Context

The M1 plan referenced `eventcore + eventcore-sqlite` for event persistence.
The eventcore library provides a full event-sourcing aggregate + command model
with CQRS conventions, optimistic concurrency, and a mature SQLite adapter.

For M1, the exit criterion is simply "cf_next_step round-trips a hand-seeded
work item from Claude Code." Standing up the full eventcore aggregate/command
model and SQLite schema before that is working would be premature.

## Decision

M1 ships a hand-rolled, file-per-event JSON log in
`.claude-factory/events/v1/{sequence:010}-{uuid}.json`.  
Replay at startup rebuilds the in-memory `ProjectState` by applying each
`FactoryEvent` variant in sequence order.

The four eventcore crates (`eventcore`, `eventcore-sqlite`, `eventcore-macros`,
`eventcore-memory`) remain declared in `[workspace.dependencies]` but are
**not** referenced from any crate's `[dependencies]` until M2.

## Consequences

- M1 ships faster and the kernel can be proven end-to-end before the more
  complex persistence layer is added.
- The hand-rolled log uses the exact same on-disk layout (one JSON file per
  event in `.claude-factory/events/v1/`) that the architecture specifies, so
  migrating to eventcore in M2 only changes the write/read path, not the file
  format or replay logic.
- No SQLite dependency in cfk-engine until M2; the operational cache is
  intentionally absent in M1.
