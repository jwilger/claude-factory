---
name: Factory Conductor
description: This skill should be used when running the Claude-Factory conductor loop, understanding how cf_next_step output should be interpreted, or troubleshooting why the loop is not advancing. It explains the dispatcher pattern, how to interpret kernel step responses, and how to dispatch Claude agents vs codex GPT executors.
version: 1.0.0
---

The conductor is a dumb dispatcher. The kernel (`cfk`) is the program; the conductor is its runtime.

## The loop

```
while true:
  step = cf_next_step()
  if step.action == "idle": display status; stop
  if step.action == "ask_human": present to user; collect answer; cf_decide(step.id, answer)
  if step.action == "spawn_agent": dispatch agent (see below); cf_submit(step.id, result)
  if step.action == "run_check": wait for cf_run_check to complete; cf_submit(step.id, result)
```

Never embellish the kernel's prompt. Never reroute to a different agent. Never skip cf_submit. The kernel's decision is authoritative.

## Dispatching agents

`cf_next_step` returns an executor spec: `{provider, model, agent_name, prompt, output_schema}`.

**provider: claude** → Use the Agent tool. Set the model per the spec (inherit/haiku/sonnet/opus). Pass the kernel's prompt verbatim. If output_schema is present, the agent must return JSON matching that schema.

**provider: codex** → Run `scripts/codex-runner.sh <model> <effort> <schema-file> <prompt-file> <output-file>` via Bash. Write the prompt to a temp file first. Read the output file after the script exits. Pass the parsed JSON to cf_submit.

## Handling cf_submit failures

If cf_submit returns a validation rejection, display the kernel's error message and stop. Do not retry automatically. The kernel's validation is authoritative — a rejection means the artifact did not meet the gate's requirements. The human must review and decide how to proceed (usually: restart the step that produced the rejected artifact).

## Parallel dispatch

For phases that allow parallel work items (e.g., multiple slices in development simultaneously in separate worktrees), cf_next_step may return multiple steps. Use the Agent tool's background mode and run them in parallel. Each must be submitted independently with its own step_id.

See `references/` for worked examples of the conductor loop for each phase.
