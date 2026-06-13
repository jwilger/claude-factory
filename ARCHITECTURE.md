# Architecture

This document is automatically projected by Claude-Factory from the
accepted Architecture Decision Records. Do not edit by hand.

## Accepted Decisions

### ADR-0001: Adopt eventcore 0.9 with the eventcore-fs (jsonl) adapter as cfk's event store

**Supersedes:** ADR 0008 in full (the M1 store-selection decision; on acceptance 0008's Status field is changed to "Superseded by 0010" and nothing else in 0008 is touched, per the ADR-immutability rule). **Amends:** ADR 0009 (stdio transport stands; only the concurrency rationale in 0009's Decision section is replaced). Canonical full text with all subsections: docs/decisions/0010-eventcore-fs-event-store.md.

## Context

cfk is moving to a multi-session model: a conductor session plus one or more interactive phase sessions, each its own cfk-mcp process against the same project directory. Two M1 defects block this: (1) append_event takes a caller-computed sequence and does plain fs::write, so two processes computing the same sequence write divergent {seq}-{uuid}.json files at the same position — a silent, undetected history fork; (2) the MCP server loads state once at startup into Arc<RwLock<ServerState>>, so events written by other processes are never observed. A planned event-log snapshot ADR (work item c57429dc) was motivated by flat-file replay latency; its root cause is the hand-rolled store itself.

eventcore 0.9 shipped the eventcore-fs adapter: a file-based, git-mergeable event store. Each append_events transaction is persisted as one immutable JSONL file under <root>/events/, named by UUID7; StreamVersion and global order are computed at read time by linearizing a transaction DAG. Immutable, uniquely-named files make a git merge of two clones' events/ directories a pure additive union; divergent branch histories surface as forks (Fork/ForkContext/DanglingTransaction) reconciled via resolvers. Same-machine writes are coordinated by the adapter's advisory leadership lock (FileLeadershipGuard); FsyncPolicy controls durability. Defect 1 dissolves (content-addressed UUID7 filenames, no sequence to collide on); Defect 2 dissolves (state computed at read time, no cached projection). A prior never-merged draft proposing eventcore + eventcore-sqlite with a git-tracked JSON export reconciled per call is preserved in the ADR appendix (recorded, not deleted).

## Decision

Single store, single source of truth: eventcore 0.9 + eventcore-fs 0.9 (FileEventStore) is cfk's only event store. No SQLite, no operational cache, no export step, no reconciliation cycle. The git-tracked JSONL transaction files are the canonical history.

On-disk layout: FsConfig root at .claude-factory/events/v1/; the adapter owns the internal events/ layout of immutable UUID7-named JSONL files, committed to git.

Stream topology: one stream per work item (work-item::{slug}) plus one project stream. Multi-stream commands (e.g. cf_claim) declare all streams; eventcore commits them atomically and eventcore-fs writes them as one immutable transaction file, so command atomicity survives to the git boundary. Slices interact only through streams/events via the store; the store is platform infrastructure, not a slice.

FCIS placement: imperative shell = all FileEventStore calls, locking, fsync, read-time linearization that touches the filesystem, and the importer; functional core = eventcore command decide-steps (pure, over already-loaded state) and the projection fold. No I/O in the core.

Per-call state reconstruction: every MCP call relinearizes from the store at the shell boundary then runs pure projection; no cached projection between calls (fixes Defect 2).

Error handling (railway-oriented): Fork/ForkContext, DanglingTransaction, lock contention/timeout, parse failure, and importer conflict are all typed Result/error values — no panics or exceptions for control flow in the core; strict-lint panic/expect bans hold with #[expect] only.

Semantic types: StreamId, WorkItemSlug, TransactionId (UUID7), StreamVersion are wrapper types in the core; the raw "work-item::{slug}" / "project" string form is the serialization encoding at the I/O boundary only.

Multi-session concurrency: same-machine writes serialized via FileLeadershipGuard (immutable per-transaction files mean no collision even absent the lock); cross-branch/offline divergence reconciled by git merge as an additive union, with forks detected and resolved.

Fork-resolution policy: an unresolved Fork or DanglingTransaction puts the project in a blocked human-decision state — the kernel surfaces divergent branches for John's resolution and never silently picks a winner or discards a branch; a rejected branch is recorded resolved-against, never deleted (consistent with review-agent-autonomy and rejected-decisions-recorded). Concrete cf-tool surface tracked in backlog; until it exists a fork blocks the project.

Amendment to ADR 0009: stdio transport stands. The three concurrency claims in 0009's Decision section (confirmed verbatim: monotonic sequence guard; per-call reload; PIPE_BUF) were false against the M1 impl. Corrected rationale: writes serialized by the adapter's advisory lock into a single-writer immutable-file backend; state recomputed per call; cross-branch divergence reconciled by git-merge fork detection. 0009's "snapshot optimization (ADR to be written)" is resolved here — linearization index serves reads, checkpoints if ever needed. 0009 revisit thresholds unchanged.

ADR relationship maintenance: on acceptance, 0008 Status -> "Superseded by 0010" (Status field only); 0009 gets a Status annotation noting its concurrency rationale is amended by 0010 (body untouched).

Migration: a one-shot restartable importer replays existing {seq}-{uuid}.json files in order through the eventcore-fs store. Idempotent by UUID. Same-sequence/different-UUID M1 divergence (Defect 1) is imported as a fork routed to human resolution — never silently picked. Old flat-log files are removed only in the final commit (after the store is fully populated), so an interrupted run loses nothing and re-running completes.

Testing: all store/fork/importer tests exercise the real eventcore-fs adapter against tempdirs (and throwaway git repos for merge tests) — no mocking libraries, driven through TDD-with-review-gates.

Performance targets (representative history of low hundreds of transactions, default FsyncPolicy, commodity hardware): cold MCP call < 100 ms; pre-commit guardrail (file-validation only, no store open) < 10 ms. Checkpoints are the escape hatch if linearization approaches the budget.

Pre-commit hook: supersedes/replaces the current event-staging hook (single hook, one failure semantics). Validates that staged transaction files parse (hard typed error otherwise) and that no store file is left unstaged; it does not recompute order/versions and does not hard-fail on a DanglingTransaction (legitimate transient merge state).

## Consequences

Positive: multi-session correctness (no single-machine fork; cross-branch divergence reconciled not lost); always-current state; one store with no duality, eliminating the reconciliation bug-class by construction; git-native diffable history with nothing to gitignore or provision externally; simpler tests (no external state dir, no mocking); planned snapshot ADR superseded without implementation; command atomicity survives the git boundary.

Negative / constrained: largest refactor since M2 (events.rs, loader.rs, server.rs startup load, every emit site in commands.rs, plus the importer); a genuinely new fork-resolution surface must be designed and built (until then a fork blocks the project); read-time linearization cost grows with history (bounded; checkpoints if ever needed); dependency on a pre-1.0 in-house adapter (eventcore-fs 0.9) — acceptable since it is John's crate, but exact versions must be pinned via cargo tooling and the strict-lint bar held across the migration.

## Appendix — superseded alternatives (recorded, not deleted)

A1. eventcore + eventcore-sqlite with git-tracked JSON export (never merged): proposed JSON exports as canonical mirrored from a derived SQLite cache, reconciled per call via sync/prune under an fs4 lock, with rebuild-on-divergence and one-file-per-commit export naming. Not chosen because every one of those mechanisms existed only because the canonical record (JSON in git) and the operational store (SQLite) were two copies needing reconciliation. eventcore-fs makes the git-tracked JSONL files the store directly — no second copy — collapsing the cache/export-divergence, crash-window, partial-export, and branch-prune bug-class to nothing. Preserved so the rejected direction and its rationale are not rehashed absent a substantive context change.

