---
id: "0001"
title: "Dedicated MCP server as the factory kernel"
status: accepted
date: 2026-06-11
---

## Context

Claude-Factory needs a deterministic orchestration layer that owns phase state machines, gates (e.g., test-review veto loops), slice queues, leases, audit log, and durable cross-session state. The key design question is where this layer lives.

Three candidates were considered:
1. A purpose-built MCP server (the kernel)
2. Claude Code Workflow scripts + state files
3. Hybrid: thin MCP for state + Workflows for in-session fan-out

## Decision

**Use a dedicated MCP server (`cfk`) as the factory kernel.**

The kernel owns all factory state durably on disk/git (event-sourced), enforces all gates and transitions, and exposes tools (`cf_next_step`, `cf_submit`, `cf_gate`, `cf_run_check`, etc.) to the conductor session. Claude Code agents become pure "workers" the kernel dispatches.

## Consequences

- Maximum determinism: every state transition is a kernel-executed function call, not an LLM inference
- Durable state survives Claude Code restarts (event-sourced replay)
- Session-per-phase concurrency and scheduled cloud routines are natural graduations — the kernel already owns the authority
- More up-front build work than Workflow-only approach; kernel must be bootstrapped before the factory can enforce its own process
- The kernel itself is built under the factory's own engineering constraints (FCIS, semantic types, ROP, strict linting) — it demonstrates and validates the methodology
