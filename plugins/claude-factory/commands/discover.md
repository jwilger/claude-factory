---
description: Run the discovery phase — socratic product discovery dialogue producing a product brief and list of workflows to model.
allowed-tools: Agent, mcp__claude-factory__cf_next_step, mcp__claude-factory__cf_claim, mcp__claude-factory__cf_release, mcp__claude-factory__cf_submit, mcp__claude-factory__cf_discovery_submit, mcp__claude-factory__cf_discovery_approve, mcp__claude-factory__cf_escalate, mcp__claude-factory__cf_decide, mcp__claude-factory__cf_record_outcome, AskUserQuestion
---

Run the Claude-Factory discovery phase.

Equivalent to `/claude-factory:work --phase discovery` — drive the conductor loop (see the Factory Conductor skill) scoped to discovery.

This phase is **interactive** (a human-decision phase). The `discovery-partner` agent runs a socratic dialogue to:
1. Draft a product brief (problem, users, value proposition)
2. Address the four risks (value, usability, feasibility, viability)
3. Enumerate the workflows/user-journeys the product must support

Submit the brief via `cf_discovery_submit`. The kernel then surfaces a human approval gate (`ask_human`): present the brief to the operator with `AskUserQuestion` and submit their decision via `cf_discovery_approve`. On approval the kernel enqueues each workflow for event modeling.
