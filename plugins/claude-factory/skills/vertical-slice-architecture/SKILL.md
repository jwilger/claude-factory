---
name: Vertical Slice Architecture
description: This skill should be used when the user or an agent is organizing code, deciding where a new piece of functionality belongs, or reviewing code for architectural correctness in a Claude-Factory managed project. It explains the three-layer structure (platform → vertical slices → application) and the strict boundaries between slices.
version: 1.0.0
---

Claude-Factory projects are organized in three layers:

## The three layers

**Platform layer**: non-business-rule-specific infrastructure shared across all slices.
- Shared event schemas (events are the only cross-slice contract)
- I/O adapters (database connections, HTTP clients, message bus interfaces)
- Shared semantic types that are not slice-specific (e.g., `UserId`, `Timestamp`)
- No business rules; no slice-specific logic

**Vertical slices**: everything needed for one behavior, end to end.
- Commands (for state_change slices)
- Event handlers / projectors (for state_view slices)
- Read models
- UI components (screens, forms) belonging to this behavior
- Business rules, domain logic
- Tests for all of the above
- No dependency on sibling slices (only on the platform layer and shared events)

**Application layer**: ties everything together.
- Launches processes, configures dependency injection, wires slices to infrastructure
- No business rules; no domain logic
- Depends on both platform and slices

## The strict boundary

A slice may depend on the platform layer. The application layer may depend on slices and the platform layer. Slices must not depend on other slices. The only cross-slice contract is shared events (defined in the platform layer).

If slice A needs to react to something slice B does, it does so by consuming an event emitted by slice B — not by calling slice B's code directly.

## Why this structure

Vertical slices can be built, tested, deployed, and replaced independently. A bug in slice A cannot break slice B (unless the shared event schema changes). This enables the factory's overlapping-phases model: multiple slices can be in development simultaneously without file conflicts.
