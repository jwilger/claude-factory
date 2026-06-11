---
id: "0007"
title: "emc installs from crates.io (with git fallback)"
status: accepted
date: 2026-06-11
---

## Context

emc (Event Model Compiler) is a Rust MCP server developed by the same author as Claude-Factory. It is not currently published on crates.io (publication is in progress). The factory's plugin needs to bootstrap emc for users.

## Decision

**The `plugins/emc/scripts/bootstrap-emc.sh` script installs emc via `cargo install emc` (crates.io), with a git fallback to `cargo install --git https://git.johnwilger.com/Slipstream/emc` until the crates.io publication completes.**

The script checks for the existence of `lake` and `quint` binaries after installation and emits a clear error with a pointer to `docs/SETUP.md` if they are missing, since `emc verify` (and therefore the event-modeling gate) requires both.

emc runs as `emc mcp stdio` and is configured as such in `plugins/emc/.mcp.json`.

## Consequences

- Once emc is on crates.io, the git fallback becomes dead code and can be removed
- The verification gate is only available when `lake` + `quint` are on PATH
- emc's operational SQLite state lives outside the repo (`$XDG_STATE_HOME/emc/...`); the model artifacts (`model/events/v1/*.json`, `model/lean/`, `model/quint/`) are git-tracked in the product repo
- `emc check` can rebuild all artifacts from the event history — drift is detectable
