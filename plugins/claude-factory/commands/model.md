---
description: Run the event modeling phase — agent drives emc MCP tools to create and formally verify event models for queued workflows.
allowed-tools: Agent, mcp__claude-factory__cf_next_step, mcp__claude-factory__cf_submit, mcp__claude-factory__cf_run_check, mcp__emc__check_project, mcp__emc__verify_project
argument-hint: "[--workflow <workflow-id>]"
---

Run the Claude-Factory event modeling phase ($ARGUMENTS).

Equivalent to `/claude-factory:work --phase event-modeling`.

The event modeling phase uses the `event-modeler` agent to drive emc MCP tools, building out the event model for each queued workflow. The gate is `emc verify_project` producing a `WorkflowReadinessDeclared` event — a fully deterministic, formally verified check. On verification, the kernel ingests the slice backlog for development.
