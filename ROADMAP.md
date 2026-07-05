# Claude-Factory — Product Roadmap

Claude-Factory is a Claude Code plugin marketplace that turns Claude Code into the console for a **dark software factory**: a deterministic orchestration kernel drives stochastic LLM agents through a six-phase, overlapping product process (discovery → event modeling → architecture → design system → TDD development → review), producing line-of-business software on any stack under non-negotiable engineering constraints.

> **Naming:** the product is **Claude-Factory**. Plugin: `claude-factory`; kernel binary: `cfk`; per-repo state dir: `.claude-factory/`; MCP tool prefix: `cf_`.

---

## Core principle: the inversion

The conductor session is a **dumb dispatcher**. The kernel is the program; agents are subroutines.

```
loop:
  step = cf_next_step()                   # kernel: deterministic state machine
  match step:
    spawn_agent  → Agent tool / Workflow (Claude tiers) or scripts/codex-runner.sh (GPT)
    ask_human    → AskUserQuestion / interactive dialogue, submit answer
    idle         → report status, stop (or wait)
  cf_submit(step.id, result)              # kernel validates evidence, advances state machine
```

Everything deterministic — running tests/linters, parsing results, diffing error progressions, projecting documents, polling PRs, routing, leasing — happens **inside the kernel process**, never via LLM. LLM agents only produce artifacts (code, tests, reviews, briefs, models) that the kernel validates and stores as evidence.

---

## Hard engineering constraints (immutable baseline)

These apply to all product code the factory builds. ADRs may extend but never contradict them.

- **Event modeling** via emc; event-sourced systems planned as vertical slices
- **Functional-core / imperative-shell**; effects pattern (native or step/trampoline) for I/O from the core
- **Railway-oriented programming** (Wlaschin) for errors
- **Semantic types only** outside I/O boundaries; "Parse, don't validate" (serde-style serialization teaching is fine)
- **Strictest-possible linting** first, narrowly relaxed only when forced
- **Enforced red-green-refactor** with independent test review and implementation review gates; narrowest-change implementation; recursive drill-down to tighter tests
- **Behavioral tests only**; no mocking libraries — substitute I/O implementations behind identical interfaces
- **Atomic Design** (quarks → atoms → molecules → organisms → templates → pages) for UI
- **Strict vertical slice architecture**: platform layer (incl. shared event schemas) / vertical slices / application layer

**Priorities:** correctness → cost → speed.

---

## Architecture

See [docs/decisions/](docs/decisions/) for the ADRs that capture the seven founding decisions.

### Repository layout

```
claude-factory/
├── .claude-plugin/marketplace.json      # marketplace: claude-factory
├── flake.nix                            # devshell: Rust nightly, lake, quint, dev tooling
├── ROADMAP.md                           # this file
├── docs/
│   ├── SETUP.md                         # toolchain deps for macOS/Linux, with & without Nix
│   └── decisions/                       # factory-development ADRs
├── kernel/                              # Rust workspace: the Claude-Factory kernel
│   ├── crates/cfk-core/                 #   pure functional core (no I/O)
│   ├── crates/cfk-engine/               #   imperative shell: event store, forge clients
│   └── crates/cfk-mcp/                  #   rmcp stdio server binary `cfk`
├── plugins/
│   ├── claude-factory/                  # the main plugin
│   │   ├── .claude-plugin/plugin.json
│   │   ├── .mcp.json
│   │   ├── commands/                    # /claude-factory:init,status,work,discover,model,architect,design,develop,review
│   │   ├── agents/                      # phase worker agents
│   │   ├── skills/                      # methodology skills
│   │   ├── hooks/hooks.json
│   │   └── scripts/                     # bootstrap-cfk.sh, codex-runner.sh
│   └── emc/                             # packaging plugin: emc MCP + bootstrap
│       ├── .claude-plugin/plugin.json
│       ├── .mcp.json
│       └── scripts/bootstrap-emc.sh
└── examples/toy-product/                # integration-test product
```

Product repos get: `.claude-factory/` (kernel state, event-sourced JSON, git-tracked), `model/` (emc), `.claude/claude-factory.local.md` (per-project settings).

### Kernel MCP tool surface

| Tool | Purpose |
|---|---|
| `cf_init` | Initialize `.claude-factory/` in a product repo |
| `cf_status` | Compact dashboard: per-phase WIP, ready work, blocked items |
| `cf_next_step` | THE tool — returns next ready instruction with executor, prompt, output schema, lease |
| `cf_submit` | Submit step result + evidence; kernel validates, advances or rejects |
| `cf_claim` / `cf_release` | Leases for multi-session coordination |
| `cf_gate` | Record review verdict (approve/veto); kernel enforces reviewer ≠ author |
| `cf_run_check` | Kernel runs deterministic check (tests, linter, build), records parsed results |
| `cf_escalate` / `cf_decide` | Surface human decision; record as event |
| `cf_backlog` | Inspect/reorder work items; ingest slices from emc verified model |
| `cf_route` | Inspect/override routing table |

### Routing table defaults

| Work type | Default executor |
|---|---|
| Socratic discovery | claude opus/fable |
| Event-model authoring | claude sonnet |
| ADR drafting | claude sonnet |
| Outer behavioral test writing | claude sonnet |
| Test review (adversarial) | **codex GPT high effort** (cross-family: different blind spots) |
| Narrowest-step implementation | claude sonnet |
| Implementation review | codex GPT or claude opus |
| Mechanical transforms | claude haiku |
| PR comment triage | claude sonnet |
| Research | claude haiku/sonnet + web |

### Development slice state machine

```
claim → write_outer_test → test_review_gate ──veto──▶ revise_test (loop)
            │ approve
            ▼
        red_check (kernel runs test, verifies fails for expected reason)
            ▼
   ┌─ implement_step (agent addresses ONLY the first visible error)
   │       ▼
   │   kernel runs tests → compares failure progression
   │       ├── new-first-error / progress → loop implement_step
   │       ├── needs >1-function change → push child TDD frame (tighter unit test)
   │       └── green → exit loop
   ▼
refactor_and_review: implementation review gate ──veto──▶ revise (loop)
            │ approve
            ▼
lint_format_gate (kernel-run) → commit → slice_done → open PR (→ Review phase)
```

The drill-down stack is explicit kernel state (`Vec<TddFrame>`) — restarts resume mid-recursion.

---

## Milestones

### M0 — Repo scaffolding ✓ (current)
Marketplace + plugin skeletons, ROADMAP.md, ADRs 0001–0007, `flake.nix` devshell, `docs/SETUP.md`.

**Exit:** marketplace installs locally; `nix develop` provides `cargo +nightly`, `lake`, `quint`.

### M1 — Kernel core
Rust workspace (cfk-core/cfk-engine/cfk-mcp); event-sourced store; work items, leases, routing table; `cf_init/status/next_step/submit/run_check`; bootstrap-cfk.sh wired into `.mcp.json`.

**Exit:** `cf_next_step` round-trips a hand-seeded work item from Claude Code.

### M2 — Development state machine (the proof)
Full TDD slice machine: gates, drill-down stack, red/green verification, lint gate, worktree handling. Agents: test-writer, test-reviewer, implementer, implementation-reviewer. codex-runner.sh. Skills: tdd-protocol, semantic-types, fcis-and-effects (v1). Against `examples/toy-product`.

**Exit:** slice goes claim→merged-commit with at least one veto loop and one drill-down; zero kernel-state corruption across mid-slice restart.

### M3 — emc integration
plugins/emc bootstrap; modeling phase machine; event-modeler agent; deterministic slice ingestion from verified model.

**Exit:** model workflow in emc, verify formally, slices appear in dev backlog, one built by M2 machinery.

### M4 — Review phase
Forge adapter (Gitea first, GitHub second); PR shepherd polling, comment-triage, merge gate.

**Exit:** factory opens PR, responds to planted review comment, merges when green.

### M5 — Discovery, Architecture, Design-system phases
Discovery dialogue + human gate; ADR lifecycle + ARCHITECTURE.md projection; atomic-design inventory + event-model cross-check. Corresponding agents + skills.

**Exit:** each phase runs individually on toy-product.

### M6 — Walking skeleton complete
One feature: discovery → merged slices via `/claude-factory:work`. Overlapping WIP proves cycling-phases model.

**Exit:** full traversal documented as runbook.

### M7 — Methodology depth & routing tuning
Distill Dilger book into skills (original text); per-stack strict-lint recipes; instrument veto rates + tokens/slice; tune routing defaults.

**Exit:** routing table defaults justified by recorded measurements.

### M8 — Concurrency graduation
Lease hardening; session-per-phase runbook; first scheduled routine (PR shepherd); statusline dashboard; HTTP mode for multi-session.

**Exit:** two simultaneous sessions + one routine without state conflicts.

### M9 — Factory builds the factory
`.claude-factory/` in this repo; factory's own backlog; metrics dashboards; remaining parts retrofitted slice by slice.

**Exit:** next kernel feature delivered by Claude-Factory itself.

---

## Verification strategy

- **Kernel:** behavioral tests per state machine (given events → command → expected events); engine tests swap SQLite-in-memory + fake forge/command-runner behind identical traits (no mocks). Property tests for lease/transition invariants.
- **Plugin integration:** `examples/toy-product` is the standing integration harness; each milestone exit criterion is a scripted end-to-end scenario (eventually run by `cf_run_check`).
- **Restart durability:** kill session mid-slice; assert replay resumes identical step (M2 exit criterion, re-checked every milestone).
- **Plugin validity:** plugin-dev validator on every plugin change.

---

## Open items (future ADRs)

- emc crates.io publish in progress — bootstrap falls back to `cargo install --git https://github.com/jwilger/emc` until then
- Lean4 + Quint runtime deps for `emc verify` — covered by `flake.nix`; product repos need same via own flake or SETUP instructions
- `codex exec` version pinning in codex-runner.sh
- Agent-frontmatter effort control for Claude agents — confirm during M2
- Gitea API coverage for required checks/reviews — confirm during M4
- Book-derived skills must be original distillations (copyright)
