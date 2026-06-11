---
description: Run the architecture phase — draft and review ADRs; project accepted decisions into ARCHITECTURE.md.
allowed-tools: Agent, mcp__claude-factory__cf_next_step, mcp__claude-factory__cf_submit, mcp__claude-factory__cf_gate, mcp__claude-factory__cf_decide
---

Run the Claude-Factory architecture phase.

Equivalent to `/claude-factory:work --phase architecture`.

The architecture phase manages the ADR lifecycle: proposed → reviewed (adr-reviewer agent checks for conflicts with the immutable baseline) → human approval gate → accepted. ARCHITECTURE.md is kernel-rendered from accepted ADRs — never LLM-written.
