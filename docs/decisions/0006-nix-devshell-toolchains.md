---
id: "0006"
title: "flake.nix devshell manages all toolchains"
status: accepted
date: 2026-06-11
---

## Context

The factory kernel requires Rust nightly, and emc's `verify` command requires `lake` (Lean4) and `quint` at runtime. These are non-trivial toolchain dependencies that need to be reproducible across developer machines and CI.

## Decision

**Maintain a `flake.nix` in this repository that provides a `devShell` with all required toolchains:**
- Rust nightly via fenix or rust-overlay
- cargo tooling: clippy, rustfmt, cargo-nextest
- `lake` (Lean4) for emc formal verification
- `quint` for emc behavioral verification
- `jq` and forge CLIs (added as adopted)

Additionally, `docs/SETUP.md` documents manual setup for macOS and Linux without Nix (rustup nightly, elan for Lean4/lake, `npm i -g @informalsystems/quint`, `cargo install emc`).

The emc bootstrap script checks for `lake` and `quint` availability and points to SETUP.md if missing, degrading gracefully (authoring works; the verification gate requires the full toolchain).

## Consequences

- `nix develop` gives a fully reproducible environment for contributors
- Non-Nix users can follow SETUP.md for manual setup
- Product repos using the factory need their own toolchain setup (own flake or following SETUP.md)
- The verification gate in the event-modeling phase requires `lake` and `quint` — documented clearly
