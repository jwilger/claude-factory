---
id: "0005"
title: "Kernel-owned routing table for model/provider selection"
status: accepted
date: 2026-06-11
---

## Context

The factory needs to invoke LLM agents with different providers (Claude tiers via the Claude Code Agent tool, GPT models via `codex exec`), models, and effort levels depending on the work type. Three approaches were considered:

1. Kernel-owned routing table (work-type → provider/model/effort), user-editable
2. Fixed roles hardcoded in agent definitions
3. A cheap router model that dynamically classifies tasks and selects executors

## Decision

**The kernel owns a routing table (TOML config) that maps work types to executor specifications.**

Default routing ships with the plugin; users can override per-project via `.claude/claude-factory.local.md`. The kernel's `cf_next_step` response includes the resolved executor so the conductor has no routing logic.

Cross-family review is an explicit design goal: test reviews and implementation reviews default to `codex exec` (GPT) rather than Claude, ensuring reviewers don't share the author's blind spots.

`codex exec` is used for GPT invocations because it uses ChatGPT subscription billing (not metered API tokens) and supports structured output via `--output-schema`. The `claude` CLI is **never** shelled out to (it forces metered API usage).

## Consequences

- Routing is deterministic and auditable
- Defaults can be tuned with measurements (veto rates, tokens/slice) — targeted for M7
- New model tiers can be added by editing TOML, not code
- `codex exec` version must be pinned/checked in `scripts/codex-runner.sh`
- Effort-level control for Claude agents (beyond model selection) needs confirmation during M2
