---
description: Run the event modeling phase — agent drives emc MCP tools to create and formally verify event models for queued workflows.
allowed-tools: Agent, mcp__claude-factory__cf_next_step, mcp__claude-factory__cf_claim, mcp__claude-factory__cf_release, mcp__claude-factory__cf_submit, mcp__claude-factory__cf_run_check, mcp__claude-factory__cf_record_outcome, mcp__emc__check_project, mcp__emc__verify_project, AskUserQuestion
argument-hint: "[--workflow <workflow-id>]"
---

Run the Claude-Factory event modeling phase ($ARGUMENTS).

Equivalent to `/claude-factory:work --phase event-modeling` — drive the conductor loop (see the Factory Conductor skill) scoped to event modeling.

The `event-modeler` agent drives the emc MCP tools (it holds the emc authoring tools in its own definition) to build the event model for each queued workflow. The gate is `emc verify_project` producing a `WorkflowReadinessDeclared` event — a deterministic, formally verified check; on verification, `cf_next_step` reconciliation seeds each verified slice into the promotion chain (ADR 0011).

This phase is **lightly interactive**: follow the operator's lead — surface consequential modeling choices via `AskUserQuestion`, otherwise proceed.
