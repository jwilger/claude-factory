---
description: Run the development phase — enforced red-green-refactor TDD loop with independent test and implementation review gates.
allowed-tools: Agent, Workflow, Bash, mcp__claude-factory__cf_next_step, mcp__claude-factory__cf_submit, mcp__claude-factory__cf_gate, mcp__claude-factory__cf_run_check, mcp__claude-factory__cf_claim, mcp__claude-factory__cf_release
argument-hint: "[--slice <slice-id>]"
---

Run the Claude-Factory development phase ($ARGUMENTS).

Equivalent to `/claude-factory:work --phase development`.

The development phase implements vertical slices from the emc-verified backlog through the enforced TDD loop:

```
claim → write_outer_test → test_review_gate (veto loops)
  → red_check (kernel verifies failure)
  → implement_step loop (narrowest change per error; drill-down on multi-function failures)
  → implementation_review_gate (veto loops)
  → lint_format_gate (kernel-run)
  → commit → open PR
```

All work happens in git worktrees (isolation). The drill-down stack is kernel state — restarts resume mid-recursion.
