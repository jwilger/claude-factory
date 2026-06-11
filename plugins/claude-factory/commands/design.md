---
description: Run the design system phase — build Atomic Design components needed for screens in the verified event model.
allowed-tools: Agent, mcp__claude-factory__cf_next_step, mcp__claude-factory__cf_submit, mcp__claude-factory__cf_gate
---

Run the Claude-Factory design system phase.

Equivalent to `/claude-factory:work --phase design-system`.

The design system phase cross-checks screens/views in the verified event model against the existing Atomic Design component inventory (quarks→pages). The kernel generates work items for each missing component. Gate: component exists + review approval.
