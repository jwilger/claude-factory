# Adopt eventcore 0.9 with the eventcore-fs (jsonl) adapter as cfk's event store

**Date:** 2026-06-13
**Status:** Proposed
**Supersedes:** ADR 0008 in full (the M1 store-selection decision; on acceptance, 0008's `Status` field is changed to "Superseded by 0010" and nothing else in 0008 is touched, per the ADR-immutability rule)
**Amends:** ADR 0009 (the stdio transport decision stands unchanged; only the concurrency rationale in 0009's Decision section is replaced — see below)

---

## Context

### The multi-session requirement

cfk is moving to a model in which a conductor session and one or more interactive phase sessions run concurrently, each spawned as its own zellij tab with its own `cfk-mcp` server process, all operating against the same project directory. This topology makes the M1 event persistence design unsafe.

### Two kernel defects that block multi-session operation

**Defect 1 — silent history fork in `append_event`** (`kernel/crates/cfk-engine/src/events.rs`)

`append_event` accepts a caller-computed sequence number and performs a plain `fs::write`. The filename scheme is `{seq:010}-{uuid}.json`, so two processes that independently compute the same sequence number both write a file with that sequence prefix but different UUIDs. Both writes succeed, producing two divergent events at the same logical position. The forked history is undetected at write time and produces undefined projection state at read time. There is no sequence guard at the write site.

**Defect 2 — startup-only state load in the MCP server** (`kernel/crates/cfk-mcp/src/server.rs`)

The server calls `load_project_state` once on startup and stores the resulting projection in `Arc<RwLock<ServerState>>`. Events appended by any other process after startup are never observed by this server instance. Each process accumulates a private, diverging view of project state.

These two defects mean the implementation never matched the concurrency claims in ADR 0009's Decision section (points 2 and 3, quoted verbatim and addressed in the amendment below).

### Replay-latency and the planned snapshot ADR

A planned "event log snapshot format" ADR (backlog work item `c57429dc`) was motivated by replay latency on a growing flat-file log. That planned decision is superseded before acceptance: the root problem is the hand-rolled flat-file store, and the eventcore-fs adapter addresses replay directly via an in-memory linearization index (and, if ever needed at scale, checkpoints) rather than a bespoke snapshot format.

### Why eventcore 0.9 + eventcore-fs resolves both defects

`eventcore` is a type-driven, command-centric event sourcing library with atomic multi-stream commands. The `eventcore-fs` 0.9 adapter is a file-based, git-mergeable event store backend:

- It persists **each `append_events` transaction as one immutable JSONL file** under `<root>/events/`, named by a **UUID7**. Files are never edited after creation.
- `StreamVersion` and global order are **computed at read time** by linearizing a transaction DAG. In single-writer mode the DAG is a linear chain, so the computed order is the append order and the computed versions are contiguous.
- Because every transaction is a uniquely named, immutable file, a `git merge` of two clones' `events/` directories is a **pure additive union with no textual conflicts**. Divergent histories written on different branches surface as **forks** (`Fork`, `ForkContext`, `BranchView`) and are reconciled through resolvers (`ResolutionOutcome`); transactions whose `parent_transaction_ids` are not yet present surface as `DanglingTransaction` and resolve when the missing files arrive — they are reported, never silently dropped.
- Same-machine write coordination is provided by the adapter's coordination layer (OS advisory locks via `FileLeadershipGuard`; per-subscription locks via `FileProjectorCoordinator`), with `FsyncPolicy` controlling write durability.

Defect 1 dissolves: filenames are content-addressed UUID7 per transaction, so there is no caller-computed sequence to collide on. Defect 2 dissolves: state is computed from the store at read time, so there is no cached projection to go stale.

A prior, never-merged draft of this ADR proposed `eventcore` over the `eventcore-sqlite` adapter with a separate git-tracked JSON export reconciled against the database per call. It is recorded in the appendix (not deleted), with the reasoning for choosing `eventcore-fs` instead.

---

## Decision

### Single store, single source of truth

cfk uses `eventcore 0.9` with the `eventcore-fs 0.9` adapter (`FileEventStore`) as its **only** event store. There is no SQLite, no operational cache, no separate export step, and no reconciliation cycle. The git-tracked JSONL transaction files **are** the canonical event history. If an event is not in a committed transaction file, it never happened for the purposes of project state.

### On-disk layout

The adapter is configured (`FsConfig`) with its root at:

```
.claude-factory/events/v1/
```

Beneath that root the adapter owns the layout: an `events/` directory of immutable per-transaction JSONL files named by UUID7. cfk treats the internal layout as opaque and never writes into it directly — all writes go through `eventcore` commands. These files **are committed to git** — they are the versioned record; nothing about the store is gitignored.

### Stream topology

- One stream per work item: `stream_id = "work-item::{slug}"`
- One project-level stream: `stream_id = "project"`
- Multi-stream commands (e.g. `cf_claim`, which touches the `project` stream and a work-item stream) are expressed as `eventcore` commands declaring all required streams. `eventcore` commits all events from such a command atomically, and `eventcore-fs` writes them as **one** immutable transaction file. Atomicity therefore survives all the way to the git boundary natively: a single atomic file create makes the whole command's events canonical or none of them.
- Slices interact with one another **only through streams and events** via the store; the store is platform-layer infrastructure, not a slice, and the multi-stream command pattern is the event mechanism — not a cross-slice backdoor through shared mutable state.

### Functional-core / imperative-shell placement

The design respects the FCIS boundary:

- **Imperative shell:** all `FileEventStore` calls (open, append, read), the advisory locking, `fsync`, the read-time DAG linearization that touches the filesystem, and the migration importer. All I/O lives here.
- **Functional core:** the `eventcore` command decision functions (which receive already-loaded stream state and return events) and the projection fold that reconstructs cfk project state from an event sequence. These are pure functions over in-memory data with no I/O. `eventcore`'s command API is structured to keep the decide-step pure; cfk's command handlers must not perform I/O inside the decide-step.

### Per-call state reconstruction

Every MCP tool call reconstructs project state from the store (read-time linearization of the transaction DAG) at the shell boundary, then runs pure projection logic in the core. The server does not cache projection state between calls. This replaces the startup-only load that caused Defect 2.

### Error handling (railway-oriented)

All new failure and divergence conditions are expressed as typed values flowing through `Result`/error railways — never panics or exceptions for control flow in the core:

- **Fork detected** (`Fork`/`ForkContext`) → a typed outcome that drives the blocked human-decision state (below), not a panic.
- **DanglingTransaction** → a typed, non-fatal condition reported and tolerated as a transient merge state.
- **Lock contention / timeout** on the advisory write lock → a typed retryable error.
- **Transaction-file parse failure** → a typed hard error surfaced to the caller (and the pre-commit hook).
- **Importer conflict** (see Migration) → a typed error that routes to human resolution.

The functional core is panic-free; the strict-lint configuration's `clippy::expect_used` / `panic` bans apply to all new code, with `#[expect(...)]` (never scattered `#[allow]`) on the rare infallible-by-construction sites.

### Semantic types

Stream identity, transaction identity, and versions are semantic types in the core, not raw primitives. `StreamId`, `WorkItemSlug`, `TransactionId` (UUID7), and `StreamVersion` are wrapper types; the raw `"work-item::{slug}"` / `"project"` string form is the `eventcore-fs` serialization encoding only, produced and parsed at the I/O boundary.

### Multi-session concurrency

- **Concurrent processes on one machine** (conductor + phase sessions against the same project dir): writes are serialized through the adapter's coordination layer (`FileLeadershipGuard` advisory lock). Each transaction is its own immutable UUID7-named file, so even absent the lock there is no filename collision and no in-place mutation to corrupt; the lock provides write serialization and a single linear DAG. Read-time linearization means every call sees every committed transaction, including those written by other processes since this process started.
- **Divergent histories across git branches / offline work**: handled by `git merge` as an additive union of the `events/` directory. Concurrent transactions that extended the same stream from the same base version surface as a **fork**, which cfk must resolve (see fork-resolution policy). This is the git-stream-merging capability that makes the multi-session model safe across branches, not just across processes.

### Fork-resolution policy

A fork can only arise after a `git merge` unions histories that diverged offline; single-writer histories never fork. When cfk detects an unresolved `Fork` (or an unresolved `DanglingTransaction`) on load, it treats the project as being in a **blocked, human-decision state**: the kernel surfaces the divergent branches for John's resolution and does not silently auto-pick a winner or discard a branch. This is consistent with the factory's review-agent-autonomy rule — reconciling divergent decision history is a human decision, and a rejected branch is recorded as resolved-against, never deleted. The concrete cf-tool surface for presenting and resolving forks is tracked in the factory backlog, not specified here; until it exists, an unresolved fork blocks the project rather than being resolvable in-tool.

### Amendment to ADR 0009 concurrency rationale

The stdio transport decision from ADR 0009 stands: stdio mode is correct for M8 and until the scale threshold defined in that ADR is reached. No change to transport.

The concurrency rationale in ADR 0009's Decision section is replaced in full. The three claims made there (confirmed verbatim against 0009's Decision section) did not match the implementation:

- Claim (2) "appends new events atomically (sequential file writes with a monotonic sequence counter guard against interleaving)" — false: the M1 write site has no sequence guard.
- Claim (3) "the kernel re-loads state per MCP call in the current design" — false: the M1 server loads state once at startup. (0009 *asserted* per-call reload; the implementation never did it — exactly the defect this ADR fixes.)
- The POSIX `PIPE_BUF` atomicity argument — irrelevant to a file-per-event scheme where the hazard is sequence collision, not torn writes.

The corrected rationale: writes are serialized by `eventcore-fs`'s coordination layer (advisory leadership lock) into a single-writer backend whose transaction files are immutable and content-addressed; project state is recomputed from the store on every call via read-time DAG linearization; and cross-branch divergence is reconciled through git-merge fork detection rather than being silently lost. Two concurrent stdio processes cannot fork a single-machine history (one holds the write lock at a time) and cannot observe stale state (each call relinearizes from the store).

ADR 0009's "Snapshot optimization (ADR to be written)" is resolved by this ADR: the adapter's in-memory linearization index serves reads, and `eventcore-fs` checkpoints are available if replay cost ever becomes material at cfk's scale. No bespoke snapshot format is needed. The "When to revisit" thresholds in ADR 0009 remain unchanged.

### Maintenance of ADR relationships

On acceptance of this ADR, ADR 0008's `Status` field is changed to "Superseded by 0010" — and nothing else in 0008 is edited, consistent with the rule that merged ADRs are immutable except for the `Status` field. ADR 0009 is amended in place only to the extent of a `Status` annotation noting that its Decision-section concurrency rationale is amended by 0010; its transport decision and body are untouched.

### Migration from the M1 hand-rolled log

A one-shot importer reads the existing `.claude-factory/events/v1/{seq:010}-{uuid}.json` files (the M1 flat log) in sequence order and replays each as an `eventcore` command through the `eventcore-fs` store, producing the adapter's immutable transaction files. The old flat-log files are then removed and the new store committed in a single git commit, preserving the full history under the new format.

The importer must handle the failure modes the M1 log can already contain:

- **Idempotency:** an event whose UUID is already present in the store is skipped, so re-running the importer is safe.
- **Same-sequence / different-UUID divergence (Defect 1):** if two M1 files share a `{seq}` prefix but carry different UUIDs, the importer must **not** silently pick one. It imports both as a fork and routes to the fork-resolution policy (human decision), consistent with the no-silent-discard rule.
- **Interrupted run:** the importer is restartable. Because import is idempotent by UUID and the old flat-log files are removed only in the final commit (after all transactions are written), an interruption leaves the old log intact and a partial new store; re-running completes the import, and the destructive removal happens only once the store is fully populated. No state is lost by an interrupted run.

### Testing approach

All store-dependent and fork-path tests exercise the **real** `eventcore-fs` adapter against a temporary directory (and, for merge/fork tests, a throwaway git repository) — no mocking libraries, consistent with the behavioral-tests-only baseline. Per-call reconstruction, the fork-resolution path, and the importer are all covered through the real adapter, driven through the factory's TDD-with-review-gates flow.

### Performance targets

Measured on a representative cfk history (low hundreds of transactions) with the default `FsyncPolicy`, on commodity developer hardware:

- Cold MCP call (open store, linearize history, project state): < 100 ms
- Guardrail check (pre-commit hook, file-validation only, no store open): < 10 ms

Read-time linearization cost grows with history size (acknowledged below); the checkpoint mechanism is the defined escape hatch and its trigger threshold is to be set when a real history approaches the cold-call budget, not pre-emptively.

### Pre-commit hook

The pre-commit hook validates the git-tracked store files only; it does not open the event store or compute projections, which is what makes the < 10 ms guardrail target achievable. This **supersedes and replaces** the current event-staging pre-commit hook the plugin installs (rather than running alongside it), so there is a single hook with one failure semantics over the store files. It operates as follows:

1. Parse every staged transaction file and validate it against the JSONL transaction format. A file that does not parse is a hard failure (typed parse error).
2. Verify that no event-store file exists in the working tree without being staged (detects store files left unstaged by a partially-completed commit) — the staging guarantee the existing hook already provides.
3. Because transaction files are immutable and content-addressed, the hook does **not** recompute global order or stream versions and does **not** hard-fail on a `DanglingTransaction` whose parents are absent — a dangling reference is a legitimate transient state during a `git merge` and is resolved by the adapter at read time when the missing files arrive.

---

## Consequences

### Positive

- **Multi-session correctness.** Concurrent processes cannot fork single-machine history (advisory write lock + immutable content-addressed files), and cross-branch divergence is reconciled by git-merge fork detection rather than lost.
- **Always-current state.** Per-call read-time linearization eliminates the stale-projection failure mode from Defect 2.
- **One store, no duality.** There is no SQLite cache, no export step, and no reconciliation cycle — the entire cache/export divergence, crash-window, partial-export, and branch-prune bug-class is eliminated by construction.
- **Git-native history.** The event store is plain JSONL committed to git; merges are additive unions, history is diffable and reviewable, and there is nothing to gitignore or provision outside the repo.
- **Test simplicity.** No external state directory or `CFK_EVENT_STORE_PATH`-style provisioning; the store lives in the project tree (or a tempdir in tests) with no second copy to keep in sync, and no mocking.
- **Snapshot complexity avoided.** The planned snapshot ADR is superseded without implementation; the adapter's linearization index (and checkpoints, if ever needed) serve replay.
- **Command atomicity survives the git boundary.** One immutable file per multi-stream transaction means git history can never contain a partial command.

### Negative / constrained

- **Refactor scope.** This is the largest refactor since M2: `cfk-engine`'s hand-rolled `events.rs` and `loader.rs`, the `cfk-mcp` server's startup load (replaced by per-call reconstruction), and every event emit site in `commands.rs` must move onto `eventcore` commands and the `eventcore-fs` store. The M1 importer must be written and tested.
- **New fork-resolution surface.** The multi-branch merge path introduces a genuinely new capability — presenting and resolving forks — that must be designed and built (tracked in the backlog). Until that surface exists, a fork blocks the project rather than being resolvable in-tool.
- **Read-time linearization cost grows with history.** Order and versions are computed at read time; at cfk's scale this is well within the cold-call budget, but a very large history would eventually warrant enabling checkpoints. This is a known, bounded follow-on, not a correctness risk.
- **Dependency on a 0.x in-house adapter.** `eventcore-fs` 0.9 is new and pre-1.0. It is John's own crate, versioned in lockstep with `eventcore`, which makes the coupling acceptable, but API churn across 0.x releases is possible. Exact versions must be pinned (via `cargo add`/`cargo upgrade` tooling, not manual `Cargo.toml` edits), and the strict-lint bar (allowlist + `#[expect]`) is held across the migration.

---

## Appendix — superseded alternatives (recorded, not deleted)

### A1. eventcore + eventcore-sqlite with git-tracked JSON export (never merged)

An earlier draft of this ADR (titled "Adopt eventcore + SQLite as cfk event store with JSON export for git canonicality") proposed using `eventcore` over the `eventcore-sqlite` adapter, with git-tracked JSON export files as the canonical record mirrored from a derived SQLite operational cache, reconciled per call via a `sync_exported_events_into_sqlite` / `prune_sqlite_streams_missing_from_exported_events` cycle under an `fs4` cross-process lock, plus a "rebuild the db from the export set on any divergence" rule and a one-file-per-commit export naming scheme to preserve multi-stream atomicity across the export boundary.

**Why it was not chosen:** every one of those mechanisms — the sync/prune reconciliation, the cross-process lock around it, the crash-window between db-commit and export-rename, the partial-export hazard, and the branch-switch pruning — existed **only** because the canonical record (JSON in git) and the operational store (SQLite) were two different copies that had to be kept in sync. When `eventcore 0.9` shipped the `eventcore-fs` adapter, the git-tracked JSONL transaction files could become *the* store directly, with no second copy. That collapses the entire reconciliation bug-class to nothing, which is strictly simpler and strictly safer than the SQLite-plus-export design. The draft never merged; it is preserved here so the rejected direction and its rationale are not rehashed absent a substantive change in context.
