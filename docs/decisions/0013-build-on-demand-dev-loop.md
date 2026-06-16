# Build the cfk binary on demand; stop committing it

**Date:** 2026-06-16
**Status:** Accepted (2026-06-16)

---

## Context

The cfk kernel is a Rust binary that the claude-factory plugin launches as its MCP
server. Until now the ~15 MB release binary was **committed to git** at
`plugins/claude-factory/bin/cfk`, and a `.githooks/pre-commit` hook rebuilt it and
re-staged it whenever kernel sources were staged. Every kernel change therefore
meant: rebuild → copy/stage the binary → commit a 15 MB blob → restart/"re-cache"
the plugin. This friction is paid on every iteration and bloats history with large
binary objects.

This matters now because the M4–M9 plan builds the kernel directly and iterates on
it heavily, and because the factory is tested against a dedicated test product
rather than self-dogfooding until M9 — so fast, low-ceremony kernel iteration is
the common case.

## Decision

**The cfk binary is built on demand at MCP-server launch and is no longer tracked
in git.**

- `plugins/claude-factory/scripts/bootstrap-cfk.sh` builds cfk into a gitignored
  cache at `plugins/claude-factory/.bin/cfk` when the kernel sources change,
  detected by a hash over `kernel/**/*.rs`, `Cargo.toml`, and `Cargo.lock`. An
  unchanged tree is an instant cache hit. The build uses `cargo` if present, else
  `nix develop` automatically. All build output goes to **stderr** so it never
  corrupts the MCP JSON-RPC stdout channel.
- `plugins/claude-factory/bin/cfk` is untracked and gitignored. It remains a
  recognized *prebuilt fallback* location for consumer installs that ship a binary
  without the kernel sources.
- The lease guards resolve the binary as `.bin/cfk`, falling back to `bin/cfk`.
- The `.githooks/pre-commit` rebuild-and-commit-binary logic is removed; the hook
  now only guards the append-only event log.

Shipping the plugin to consumers who do not have the kernel sources will require
**per-platform prebuilt binaries** produced by release packaging; that is deferred
to M8 (the same milestone that introduces HTTP mode and broader distribution).

## Consequences

- Kernel iteration loses the rebuild→copy→commit→re-cache dance: edit kernel,
  restart the MCP server (or session), and the next launch rebuilds automatically.
- Git history stops accumulating 15 MB binary blobs.
- First launch after a kernel change pays the build cost (~30 s to a few minutes);
  this can approach the MCP startup window for very large rebuilds, an acceptable
  trade for the dev loop and revisited only if it becomes a problem.
- A standalone consumer install without `kernel/` sources currently needs a
  prebuilt `bin/cfk` placed manually until M8 release packaging exists.
- Documented in `docs/SETUP.md` ("Developing the kernel"). Pairs with the
  `.claude-factory/LEASE_BYPASS` escape hatch for direct kernel TDD.
