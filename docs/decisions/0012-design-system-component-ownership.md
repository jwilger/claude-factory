# Design-system phase: per-slice atomic-design gating and component ownership

**Date:** 2026-06-16
**Status:** Accepted (2026-06-16)
**Related:** ADR 0011 (per-slice promotion chain) defines the design-triage gate this ADR governs.

---

## Context

The factory builds UI under Atomic Design (quarks → atoms → molecules → organisms
→ templates → pages) and under strict vertical-slice architecture (platform layer
/ vertical slices / application layer). ADR 0011 introduces a per-slice
`DesignTriage` gate, but leaves open *what the gate decides, where the resulting
components live, and how that is recorded.* Without an explicit rule, design work
produced for one slice has no defined home and risks either duplication
(every slice rebuilds its own buttons) or boundary erosion (slice-specific screens
leaking into shared space).

## Decision

**For each slice that touches the UI, the design gate asks a single question:
"do we have the full set of quarks, atoms, molecules, organisms, templates, and/or
page designs needed to build this slice's UI?" Missing pieces are built
interactively and placed according to reuse, then recorded in the application
design guide.**

### Placement / ownership

- **Reusable elements → the platform UI library.** Any component intended to be
  shared across slices (typically lower atomic levels: quarks, atoms, molecules,
  and common organisms/templates) is owned by the platform layer.
- **Slice-specific components and pages → the vertical slice.** Components that
  exist only to serve one slice (typically slice pages and bespoke organisms) are
  owned by that slice and do not enter the shared library.
- The split follows the vertical-slice boundary already in force: shared,
  reuse-justified work goes to the platform; everything else stays local to the
  slice that needs it.

### Process

- Building the missing inventory is an **interactive** process (a UI/UX design
  agent working with the human), at least as interactive as architecture — not a
  deterministic kernel step. The kernel only deterministically decides *whether a
  slice touches the UI at all* (see ADR 0011); everything past that is judgment.
- The resulting components and their level/ownership are added to the
  **application design guide** (the project's design inventory), so later slices'
  triage can see existing coverage and fast-pass when the inventory already
  satisfies their needs.
- Design inventory entries record at minimum: atomic level, owning layer
  (platform vs the specific slice), and the slice(s) they satisfy.

## Consequences

- The design system accretes **just-in-time**: a project's first UI slices build
  the platform baseline; later slices increasingly fast-pass against existing
  inventory. This mirrors the architecture-baseline pattern in ADR 0011.
- Reuse is the default for shared elements, preventing per-slice duplication of
  common UI, while slice ownership keeps bespoke screens from polluting the shared
  library — both consistent with the vertical-slice architecture.
- The design inventory data model must carry level + ownership + satisfied-slice
  links; the empty `DesignCrossCheckCompleted` handler must be implemented so
  generated work is materialized rather than discarded.
- Triage correctness depends on an accurate, queryable design inventory; if the
  inventory drifts from the actual codebase, slices may falsely fast-pass. Keeping
  the inventory authored through the gate (not hand-edited) mitigates this.
