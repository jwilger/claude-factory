---
name: Factory Conductor
description: This skill should be used when running the Claude-Factory conductor loop, understanding how cf_next_step output should be interpreted, or troubleshooting why the loop is not advancing. It explains the dispatcher pattern, how to interpret kernel step responses, and how to dispatch Claude agents vs codex GPT executors.
version: 1.0.0
---

The conductor is a dumb dispatcher. The kernel (`cfk`) is the program; the conductor is its runtime.

## The loop

```
while true:
  resp = cf_next_step()
  if resp.status == "idle": display status; stop
  switch resp.action.type:
    ask_human    → present resp to user; collect answer; cf_decide(step_id, answer)
    spawn_agent  → dispatch the agent (see below); submit via the tool the PROMPT names
    gate_review  → dispatch the reviewer agent; cf_gate(step_id, verdict)
    run_check    → cf_run_check(check_name); the kernel records and advances
    open_pr      → open the PR for the slice; cf_pr_open(...)
    run_pr_poll  → cf_pr_poll(...)
    merge_pr     → cf_pr_merge(...)
```

Never embellish the kernel's prompt. Never reroute to a different agent. Never skip the submit step. The kernel's decision is authoritative.

## Which submit tool

A `spawn_agent` step's **prompt names the submit tool** — always follow it. `cf_submit` is the default (TDD steps, PR-comment triage); specific phases instruct otherwise:

| Prompt instructs | Phase / step |
|---|---|
| `cf_submit` | TDD test/implement, PR comment triage |
| `cf_discovery_submit` | discovery brief |
| `cf_adr_submit` | architecture ADR draft |
| `cf_triage_submit` | architecture / design **triage** gate |
| `cf_design_add_component` | design-system component build |

If you ever cannot tell which tool to call, re-read the prompt's final "Submit via …" line. Do not substitute a different tool.

## Autonomy policy (where humans are in the loop)

The kernel decides *what* runs; this policy decides *whether to pause for the operator* after a `spawn_agent` step, keyed on `resp.phase`:

- **Discovery, Architecture, Design-system → interactive.** These are human-decision phases. Run the agent to produce its analysis/recommendation, then present it to the operator (`AskUserQuestion`) and submit the operator's decision — e.g. for a triage step, pass the operator's `needs_followup` choice to `cf_triage_submit`. Never let the agent's recommendation auto-commit the decision.
- **Event modeling → lightly interactive.** Follow the operator's lead: surface modeling choices when they are consequential, otherwise proceed.
- **Development → fully autonomous.** Run the red-green-refactor loop without pausing. The only human gate in development/review is the **PR review gate** (the `gate_review` on the open PR, and final merge approval).
- **Review → autonomous** except the PR review gate above.

This is the contract: a feature flows discovery → modeled → architecture/design (interactive gates) → built autonomously → merged after the PR review gate, with the operator touched only at those planned points.

## Dispatching agents

`cf_next_step` returns an executor spec: `{provider, model, agent_name, prompt, output_schema}`.

**provider: claude** → Use the Agent tool. Set the model per the spec (inherit/haiku/sonnet/opus). Pass the kernel's prompt verbatim. If output_schema is present, the agent must return JSON matching that schema.

**provider: codex** → Run `scripts/codex-runner.sh <model> <effort> <schema-file> <prompt-file> <output-file>` via Bash. Write the prompt to a temp file first. Read the output file after the script exits. Pass the parsed JSON to cf_submit.

## Autonomous operation

The conductor runs without stopping to ask questions. Make tactical decisions yourself:

- **Missing executor script**: If `scripts/codex-runner.sh` does not exist, fall back to the equivalent Claude agent for that gate kind and continue.
- **Dependency gap**: If a test requires a public API that does not exist yet, add the missing work item to the backlog and veto the test with a clear reason, then continue the loop.
- **TDD discipline**: Write ONE failing test per `spawn_agent` step. Kent Beck TDD — simplest test that pins the core contract, then make it pass, then the next test.

Only stop when the kernel returns `idle`, when the kernel issues an `ask_human` action, or when there is a genuine blocker with no path forward (missing credentials, irreversible destructive action). Do NOT use `AskUserQuestion` for tactical decisions.

## Handling cf_submit failures

If cf_submit returns a validation rejection, display the kernel's error message and stop. Do not retry automatically. The kernel's validation is authoritative — a rejection means the artifact did not meet the gate's requirements. The human must review and decide how to proceed (usually: restart the step that produced the rejected artifact).

## Parallel dispatch

For phases that allow parallel work items (e.g., multiple slices in development simultaneously in separate worktrees), cf_next_step may return multiple steps. Use the Agent tool's background mode and run them in parallel. Each must be submitted independently with its own step_id.

See `references/` for worked examples of the conductor loop for each phase.
