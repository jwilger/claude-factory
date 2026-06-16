---
description: Run the factory conductor loop — dispatches the next ready work item across all phases. Runs continuously until idle or a human decision is needed.
allowed-tools: Agent, Workflow, Bash, Read, mcp__claude-factory__cf_next_step, mcp__claude-factory__cf_submit, mcp__claude-factory__cf_status, mcp__claude-factory__cf_gate, mcp__claude-factory__cf_escalate, mcp__claude-factory__cf_decide, mcp__claude-factory__cf_run_check, mcp__claude-factory__cf_claim, mcp__claude-factory__cf_release, mcp__claude-factory__cf_discovery_submit, mcp__claude-factory__cf_discovery_approve, mcp__claude-factory__cf_adr_submit, mcp__claude-factory__cf_triage_submit, mcp__claude-factory__cf_design_add_component, mcp__claude-factory__cf_design_cross_check, mcp__claude-factory__cf_pr_open, mcp__claude-factory__cf_pr_poll, mcp__claude-factory__cf_pr_merge, mcp__claude-factory__cf_record_outcome, mcp__claude-factory__cf_metrics, AskUserQuestion
argument-hint: "[--phase <phase>] [--once]"
---

Run the Claude-Factory conductor loop.

Arguments: $ARGUMENTS

## Loop behavior

1. Call `cf_next_step` (with `phase_filter` if `--phase` was passed) to get the next ready instruction from the kernel.
2. If the step is for a **non-development, non-review phase** and lacks an active lease, call `cf_claim` first to acquire a lease before executing.
3. Execute based on the step's `action` and `phase`:
   - `spawn_agent`: Use the Agent tool with the executor's model, agent_name from the routing spec, the kernel-provided prompt, and output_schema if present. Then call the phase-specific submission tool (see **Phase submission routing** below).
   - `gate_review`: Use the Agent tool with the reviewer agent for this gate kind. Call `cf_gate` with the verdict.
   - `ask_human`: Present the escalation to the user interactively, collect their decision:
     - For discovery approval gates: call `cf_discovery_approve` with the decision.
     - For other human decisions: call `cf_decide` with the result.
   - `run_check`: The kernel will run this itself via `cf_run_check`; poll for completion.
   - `open_pr`: Call `cf_pr_open` to have the kernel open the PR.
   - `run_pr_poll`: Call `cf_pr_poll`; the kernel will poll the forge for CI and review status.
   - `merge_pr`: Call `cf_pr_merge` once all-green.
   - `idle`: Report status summary and stop the loop.
4. After each step completes, call `cf_record_outcome` with the work item id, outcome (approved/vetoed/completed), and token count if known. This accumulates veto-rate and token-cost data for routing-table tuning.
5. If `--once` was passed, stop after one iteration. Otherwise continue from step 1.

## Phase submission routing

After a `spawn_agent` step completes, submit results via the appropriate tool:

The step's `prompt` always names the tool to call — follow it. Summary:

| Phase / step | Submission tool | Notes |
|---|---|---|
| Discovery — Dialogue | `cf_discovery_submit` | Submits the product brief JSON |
| Discovery — BriefReady (human gate) | `cf_discovery_approve` | Conductor presents brief to user, submits approval |
| Architecture — triage | `cf_triage_submit` | `needs_followup` true iff an ADR is required (see Autonomy) |
| Architecture — ADR draft | `cf_adr_submit` | Submits the ADR content |
| Design system — triage | `cf_triage_submit` | `needs_followup` true iff components must be built |
| Design system — building | `cf_design_add_component` | Submits one component; loop until no more steps |
| Development (TDD) | `cf_submit` | Generic submission for test/impl results |
| Review | `cf_submit` | Generic submission for PR-comment triage results |

## Agent dispatching

When spawning an agent, the routing spec from `cf_next_step` specifies:
- `provider: claude` → use the Agent tool with `model` set per the spec (inherit/haiku/sonnet/opus)
- `provider: codex` → invoke `scripts/codex-runner.sh <model> <effort> <schema-file> <prompt-file> <output-file>` via Bash, then read the output file and pass its contents to the appropriate submission tool

Always pass the kernel's prompt verbatim — do not embellish or compress it.

## Where the operator is in the loop

The kernel decides *what* runs; this policy decides *whether to involve the operator* after a `spawn_agent` step, keyed on the step's `phase`:

- **Discovery, Architecture, Design system → interactive.** These are human-decision phases. Run the agent to produce its analysis/recommendation, then present it to the operator with `AskUserQuestion` and submit the operator's decision. For a **triage** step, the agent recommends `needs_followup`; the operator confirms, and you pass their choice to `cf_triage_submit`. Never let an agent's recommendation auto-commit a decision in these phases.
- **Event modeling → lightly interactive.** Follow the operator's lead; surface consequential modeling choices, otherwise proceed.
- **Development → fully autonomous.** Run red-green-refactor without pausing.
- **Review → autonomous**, except the **PR review gate** (`gate_review` on the PR and final merge approval), which is a human decision.

The contract: a feature flows discovery → modeled → architecture/design (interactive gates) → built autonomously → merged after the PR review gate, touching the operator only at those planned points.

## Autonomous tactical decisions (do NOT ask the operator)

Within autonomous phases, make tactical calls yourself — these are not the planned human points above:

- **Missing executor script** (`scripts/codex-runner.sh` absent): fall back to the equivalent Claude agent for that gate kind and continue.
- **Dependency gap**: if a test requires a public API that does not yet exist, add the missing work item to the backlog and veto the test with a clear reason, then continue the loop.
- **TDD discipline**: write ONE failing test per `spawn_agent` step (Kent Beck: simplest test that pins the core contract).

Stop the loop only when the kernel returns `idle`, or on a genuine blocker with no path forward (missing credentials, irreversible destructive action). `ask_human` actions and the interactive phases above are handled in-loop via `AskUserQuestion`/`cf_decide` — they pause for input but do not stop the loop.

## Error handling

If any submission tool returns a validation failure, display the reason to the user and stop — do not retry automatically. The kernel's rejection is authoritative.
