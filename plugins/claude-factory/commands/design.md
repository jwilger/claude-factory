---
description: Run the design system phase — per-slice design triage and Atomic Design component building.
allowed-tools: Agent, mcp__claude-factory__cf_next_step, mcp__claude-factory__cf_claim, mcp__claude-factory__cf_release, mcp__claude-factory__cf_submit, mcp__claude-factory__cf_triage_submit, mcp__claude-factory__cf_design_add_component, mcp__claude-factory__cf_design_cross_check, mcp__claude-factory__cf_gate, mcp__claude-factory__cf_decide, mcp__claude-factory__cf_record_outcome, AskUserQuestion
---

Run the Claude-Factory design system phase.

Equivalent to `/claude-factory:work --phase design-system` — drive the conductor loop (see the Factory Conductor skill) scoped to design-system work items.

This phase is **interactive** (a human-decision phase). Step kinds:

- **Design triage** (`DesignTriage`, ADR 0011/0012): the `design-triage` agent decides whether a slice has a UI surface and, if so, whether the existing Atomic Design inventory (quarks → atoms → molecules → organisms → templates → pages) already covers it. Present its recommendation to the operator with `AskUserQuestion`, then submit via `cf_triage_submit` (`needs_followup` = true iff components must be built). Fast-pass advances the slice; needs-followup spawns a `DesignSystemBuild` item for the same slice.
- **Component build** (`DesignSystemBuild`): the `design-system-builder` agent builds a missing component; submit via `cf_design_add_component`. Reusable elements go to the platform UI library; slice-specific ones are owned by the slice (ADR 0012). `cf_design_cross_check` is an optional batch convenience to surface workflow-level page gaps.

Never let an agent's recommendation auto-commit a design decision; the operator decides.
