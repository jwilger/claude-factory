---
description: Run the discovery phase — socratic product discovery dialogue producing a product brief and list of workflows to model.
allowed-tools: Agent, mcp__claude-factory__cf_next_step, mcp__claude-factory__cf_submit, mcp__claude-factory__cf_escalate, mcp__claude-factory__cf_decide
---

Run the Claude-Factory discovery phase.

Equivalent to `/claude-factory:work --phase discovery`.

The discovery phase uses the `discovery-partner` agent in a socratic dialogue to:
1. Draft a product brief (problem, users, value prop)
2. Address the four risks (value, usability, feasibility, viability)
3. Enumerate workflows/user-journeys the product must support
4. Present for human approval (kernel escalation gate)
5. On approval, enqueue workflows for event modeling

The conductor loop handles dispatch — run this command to focus the work session on discovery.
