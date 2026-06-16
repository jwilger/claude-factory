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

## Comprehensive dual review (run once per PR, before the merge gate)

After the PR is open and before merging, run the **multi-front, multi-agent review** of the PR diff — both model families, surfaced through the existing comment-triage flow:

1. **codex (gpt-5.5):** run `bash ${CLAUDE_PLUGIN_ROOT}/scripts/codex-review.sh main` from the product repo (reviews the branch vs `main`; prints severity-tagged `[P#] title — file:line` findings).
2. **Opus 4.8:** dispatch the `implementation-reviewer` agent (model: opus) over the **whole PR diff** (`git diff main...HEAD`), applying the full engineering baseline.
3. Collect both families' findings. For each actionable finding, either fix it (re-enter development for that slice) or post it as a PR comment via the forge so the kernel's poll surfaces it as a `pr-shepherd-triage` item. Empirically the codex (cross-family) gate in development already catches the substantive defects and the Opus pass is a near-redundant safety net — but running both is the standard, since the two families have complementary blind spots (e.g. codex is stronger on runtime type-soundness in type-erased languages).

A PR with no surviving actionable findings, green CI, and approval is ready to merge.

## The merge gate is a human decision

The **PR review gate is the planned human point for development/review**: present the PR's state and the dual-review findings to the operator with `AskUserQuestion` before merging. Merge requirements (CI green, approvals) are enforced by the kernel; the human-approval requirement is configurable per repo in `.claude/claude-factory.local.md`.
