---
description: Run the review phase — PR shepherd polls the forge, triages review comments, and merges when all requirements are met.
allowed-tools: Agent, Bash, mcp__claude-factory__cf_next_step, mcp__claude-factory__cf_submit, mcp__claude-factory__cf_run_check
---

Run the Claude-Factory review phase.

Equivalent to `/claude-factory:work --phase review`.

The review phase shepherds open PRs: the cfk kernel polls the forge API directly (Gitea first, then GitHub), identifies CI status, review comments, and mergeability. New comments generate triage work items for the pr-shepherd-triage agent. The kernel enforces the merge gate; human-approval requirement is configurable per repo in `.claude/claude-factory.local.md`.
