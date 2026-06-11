---
id: "0002"
title: "Conductor-first concurrency, graduate to multi-session"
status: accepted
date: 2026-06-11
---

## Context

The factory's six phases (discovery, event modeling, architecture, design system, development, review) are designed to overlap and cycle. Multiple features can be at different phases simultaneously. This creates a concurrency question: how do those overlapping phases actually execute in Claude Code?

Three patterns were considered:
1. One conductor session dispatching background agents/workflows
2. Session per phase from day one (multiple terminal tabs)
3. Full dark factory with scheduled cloud routines in v1

## Decision

**Build for one interactive conductor session (M0–M6), designing the kernel with leasing and git-friendly state from day one so multi-session and cloud routines can be added without rework.**

A single `/claude-factory:work` command drives the loop. The kernel's `cf_next_step` returns the next ready instruction across all phases; the conductor dispatches background agents and surfaces escalations interactively. Phase commands (`/claude-factory:develop`, etc.) are thin wrappers — they're exactly what runs in separate tabs when the user graduates to multi-session.

## Consequences

- Simple mental model for v1: one pane of glass
- Kernel must implement claim/lease semantics from day one (low-friction graduation)
- State must be git-tracked JSON so a second session or cloud agent can read/write it
- Throughput bounded by one session until graduation — acceptable for v1
- First scheduled routine (PR shepherd) targeted for M8
