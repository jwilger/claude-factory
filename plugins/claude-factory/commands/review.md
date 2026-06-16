---
description: Run the review phase — PR shepherd polls the forge, triages review comments, runs the PR review gate, and merges when all requirements are met.
allowed-tools: Agent, Bash, mcp__claude-factory__cf_next_step, mcp__claude-factory__cf_claim, mcp__claude-factory__cf_release, mcp__claude-factory__cf_submit, mcp__claude-factory__cf_pr_open, mcp__claude-factory__cf_pr_poll, mcp__claude-factory__cf_pr_merge, mcp__claude-factory__cf_gate, mcp__claude-factory__cf_decide, mcp__claude-factory__cf_record_outcome, AskUserQuestion
---

Run the Claude-Factory review phase.

Equivalent to `/claude-factory:work --phase review` — drive the conductor loop (see the Factory Conductor skill) scoped to review.

The review phase shepherds open PRs. The kernel returns the action to take:

- `open_pr` → `cf_pr_open` (kernel opens the PR on the forge — Forgejo/Gitea, then GitHub).
- `run_pr_poll` → `cf_pr_poll` (kernel polls CI status, reviews, and new comments).
- `spawn_agent` (comment triage) → run the `pr-shepherd-triage` agent; submit via `cf_submit`.
- `merge_pr` → `cf_pr_merge` once green and approved.

The **PR review gate is a human decision** (the planned human point for development/review): present the PR's state and any review findings to the operator with `AskUserQuestion` before merging. Merge requirements (CI green, approvals) are enforced by the kernel; the human-approval requirement is configurable per repo in `.claude/claude-factory.local.md`.

Cross-family review note: the test/implementation gates already use `codex exec` (gpt-5.5) for an independent perspective; running a `codex exec`-based review of the PR diff alongside an Opus review is the intended way to gate PR quality (the interactive `codex review` subcommand is not headless-friendly).
