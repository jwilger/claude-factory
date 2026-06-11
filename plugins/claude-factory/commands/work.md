---
description: Run the factory conductor loop — dispatches the next ready work item across all phases. Runs continuously until idle or a human decision is needed.
allowed-tools: Agent, Workflow, Bash, Read, mcp__claude-factory__cf_next_step, mcp__claude-factory__cf_submit, mcp__claude-factory__cf_status, mcp__claude-factory__cf_gate, mcp__claude-factory__cf_escalate, mcp__claude-factory__cf_decide, mcp__claude-factory__cf_run_check
argument-hint: "[--phase <phase>] [--once]"
---

Run the Claude-Factory conductor loop.

Arguments: $ARGUMENTS

## Loop behavior

1. Call `cf_next_step` to get the next ready instruction from the kernel.
2. Execute based on the step's `action`:
   - `spawn_agent`: Use the Agent tool with the executor's model, agent_name from the routing spec, the kernel-provided prompt, and output_schema if present. Mark the step's lease as active.
   - `run_workflow`: Use the Workflow tool with the kernel-provided script or named workflow.
   - `ask_human`: Present the escalation to the user interactively, collect their decision, call `cf_decide` with the result.
   - `run_check`: The kernel will run this itself via `cf_run_check`; poll for completion.
   - `idle`: Report status summary and stop the loop.
3. Call `cf_submit` with the step_id and result from step 2.
4. If `--once` was passed, stop after one iteration. Otherwise continue from step 1.
5. If `--phase <phase>` was passed, only work on steps belonging to that phase.

## Agent dispatching

When spawning an agent, the routing spec from `cf_next_step` specifies:
- `provider: claude` → use the Agent tool with `model` set per the spec (inherit/haiku/sonnet/opus)
- `provider: codex` → invoke `scripts/codex-runner.sh <model> <effort> <schema-file> <prompt-file> <output-file>` via Bash, then read the output file and pass its contents to `cf_submit`

Always pass the kernel's prompt verbatim — do not embellish or compress it.

## Error handling

If `cf_submit` returns a validation failure, display the reason to the user and stop — do not retry automatically. The kernel's rejection is authoritative.
