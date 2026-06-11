---
id: "0003"
title: "Rust nightly for the factory kernel"
status: accepted
date: 2026-06-11
---

## Context

The factory kernel (`cfk`) needs a language and toolchain. It must exemplify the factory's own engineering constraints: semantic types everywhere, functional-core/imperative-shell, railway-oriented programming, and strictest-possible linting.

Three candidates were considered:
1. Rust (nightly) — same ecosystem as emc, strongest semantic-type story
2. TypeScript (Bun/Node) — fastest iteration, trivial distribution, the plugin ecosystem's lingua franca
3. Decide per-component in ADRs

## Decision

**Use Rust nightly for the factory kernel.**

Rust with `nutype` provides the strongest "semantic types everywhere" and "parse don't validate" story. The factory's own code must be a demonstration of its methodology. The nightly toolchain is selected to ensure access to the latest stable language features and to keep parity with emc's toolchain choices.

The kernel is organized as a Cargo workspace:
- `cfk-core`: pure functional core, no I/O, all business logic
- `cfk-engine`: imperative shell, event store (eventcore + SQLite), forge clients
- `cfk-mcp`: `rmcp` stdio MCP server binary

## Consequences

- Kernel code embodies the factory's own engineering rules — it's the reference implementation
- Slower to iterate than TypeScript; compensated for by the factory's TDD enforcement once M2 is complete
- Nightly Rust: must pin toolchain version in `rust-toolchain.toml` to avoid surprise breakage
- Stricter clippy configuration (`#![deny(clippy::all, clippy::pedantic)]` minimum) from the first commit
- `nutype`, `rmcp`, `eventcore`, `eventcore-sqlite`, `tera` are key dependencies
