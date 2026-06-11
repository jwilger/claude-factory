---
id: "0004"
title: "Durable full-product roadmap across sessions"
status: accepted
date: 2026-06-11
---

## Context

The factory will be built over many Claude Code sessions, potentially with restarts. Without a persistent, in-repo reference, the bigger-picture design intent is at risk of drift or loss across context compactions.

## Decision

**Maintain a durable `ROADMAP.md` in this repository as the master product plan, referenced across all sessions.**

The roadmap is written from the start (M0) and updated as milestones are completed or designs evolve. It contains the full nine-milestone plan, the immutable engineering constraints, the architecture, and open items queued as future ADRs. Milestone decisions are also recorded individually in `docs/decisions/`.

Claude Code sessions begin by reading ROADMAP.md to restore context before beginning work.

## Consequences

- Any session can resume work without re-deriving the design
- Milestone scope changes require editing ROADMAP.md (this is intentional — forces explicit decision)
- Walking skeleton (M0–M2) is built first to prove the premise; later milestones may be revised based on what is learned
- Claude Code restarts mid-milestone are explicitly acceptable; the kernel's event-sourced state provides the fine-grained resume capability
