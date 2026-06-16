---
description: Run the architecture phase — per-slice triage, ADR drafting/review, and ARCHITECTURE.md projection.
allowed-tools: Agent, mcp__claude-factory__cf_next_step, mcp__claude-factory__cf_claim, mcp__claude-factory__cf_release, mcp__claude-factory__cf_submit, mcp__claude-factory__cf_triage_submit, mcp__claude-factory__cf_adr_submit, mcp__claude-factory__cf_gate, mcp__claude-factory__cf_decide, mcp__claude-factory__cf_record_outcome, AskUserQuestion
---

Run the Claude-Factory architecture phase.

Equivalent to `/claude-factory:work --phase architecture` — drive the conductor loop (see the Factory Conductor skill) scoped to architecture work items.

This phase is **interactive** (a human-decision phase). Two step kinds appear:

- **Architecture triage** (`ArchitectureTriage`, ADR 0011): the `architecture-triage` agent assesses whether a slice forces a new/changed cross-cutting decision. Present its recommendation to the operator with `AskUserQuestion`, then submit the operator's decision via `cf_triage_submit` (`needs_followup` = true iff an ADR is required). Fast-pass advances the slice; needs-followup spawns an `AdrDrafting` item for the same slice.
- **ADR draft → review → decide** (`AdrDrafting`): the `architect` agent drafts (submit via `cf_adr_submit`); the `adr-reviewer` agent reviews for conflicts with the immutable baseline (`gate_review` → `cf_gate`); on a veto or final call, present to the operator and record via `cf_decide`. ARCHITECTURE.md is kernel-rendered from accepted ADRs — never LLM-written.

Never let an agent's recommendation auto-commit an architecture decision; the operator decides.
