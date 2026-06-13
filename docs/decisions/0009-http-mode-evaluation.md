# ADR 0009 — stdio-first kernel transport; HTTP mode deferred

**Status:** Accepted (concurrency rationale in the Decision section amended by ADR 0010; the stdio transport decision stands)  
**Date:** 2026-06-11  
**Supersedes:** —  
**Superseded by:** —

---

## Context

The cfk kernel and emc both support two transport modes:

- **stdio** — each Claude Code session spawns its own kernel process via the
  `.mcp.json` bootstrap script. The process reads its state from the
  event log on disk (`.claude-factory/events/v1/*.json`) at startup and
  keeps an in-memory projection for the session lifetime.

- **HTTP** — a single long-running kernel process exposes a bearer-token
  REST (or MCP-over-HTTP) endpoint. Multiple Claude Code sessions connect
  to the same process and share the in-memory projection directly.

M8 introduces session-per-phase concurrency (one Claude Code tab per phase).
The question is: which transport is correct for M8 and beyond?

---

## Decision

**Use stdio mode for M8 and until the scale threshold is reached.**

The shared-state problem is solved at the event-log layer, not the process
layer. Each stdio session:

1. Replays the event log at startup to reconstruct current state.
2. Appends new events atomically (sequential file writes with a monotonic
   sequence counter guard against interleaving).
3. Reads the latest log on every command (the kernel re-loads state per
   MCP call in the current design, avoiding stale projection issues).

File-system serialization at M8 throughput (≤8 simultaneous sessions,
each making ≤10 calls/minute) is well within what a local filesystem can
handle without locking issues.

---

## Consequences

**Positive:**
- Zero additional infrastructure. No persistent process to manage, monitor,
  or restart.
- Crash isolation: a crashed session's kernel process dies with it; no
  shared state is corrupted. The event log is the source of truth.
- Works identically in a developer laptop, a CI container, and a cloud IDE
  — wherever the event log directory is accessible.
- The lease protocol in the event log already prevents double-execution;
  stdio does not weaken this guarantee.

**Negative / deferred:**
- Each session replays the full event log at startup. For large, long-lived
  projects (thousands of events) this adds latency. Snapshot optimization
  (ADR to be written) will address this before it matters.
- Two sessions appending events concurrently depend on the OS file-append
  guarantee for atomicity. This holds on all POSIX systems for writes ≤
  PIPE_BUF (4 KiB); event envelopes are smaller. On networked filesystems
  (NFS, SMB) this guarantee is weaker — HTTP mode is required there.

---

## When to revisit

Switch to HTTP mode when **any** of the following is true:

- More than 8 simultaneous sessions in one product repo (replay latency
  becomes noticeable, and append contention increases).
- Sessions run in cloud-hosted environments without a shared local
  filesystem (e.g. two separate VMs editing the same repo over the network).
- A persistent kernel process is operationally desirable for other reasons
  (webhooks, push-based forge notifications, shared scheduling).

To enable HTTP mode: start `cfk serve --port <port> --token <token>` as a
background process (systemd unit, docker container, or nix service), then
update `.mcp.json` in the product repo to point to the HTTP endpoint. No
code changes required; the MCP tool surface is identical.
