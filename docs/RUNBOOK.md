# Running Claude-Factory on a product

How to drive a feature from idea to merged PR. The kernel (`cfk`) is the program;
you (via the slash commands) are its runtime. Humans are in the loop only at the
**planned points**: discovery, architecture, and UI-design decisions are
interactive; development runs autonomously; the PR review gate is a human decision.

## Prerequisites

- The `claude-factory` plugin installed (and `emc` for event modeling). The cfk
  binary builds on demand — see [SETUP.md](SETUP.md).
- A git repo for your product with a Forgejo/GitHub remote (`origin`).
- A forge token in the environment so the review phase can open/merge PRs:
  `FORGEJO_TOKEN` (aliases: `FORGE_TOKEN`, `GITEA_TOKEN`). Host/owner/repo are
  inferred from the `origin` remote; override with `FORGEJO_URL/OWNER/REPO`.
- A `.claude-factory/checks.toml` declaring how to run the product's checks, e.g.:
  ```toml
  [checks.tests] command = "cargo nextest run"     # or: pytest -q, npm test, …
  [checks.lint]  command = "cargo clippy -- -D warnings"
  [checks.build] command = "cargo build"
  ```
  The kernel runs these itself — agents never self-report pass/fail.

## One-time

```
/claude-factory:init      # creates .claude-factory/ in the product repo
```

## The flow (one feature)

The phases form a pipeline; each verified event-model slice flows through it
automatically (ADR 0011). Run the interactive phases yourself, then let `/work`
grind development autonomously.

1. **Discovery** (interactive) — `/claude-factory:discover`
   Socratic dialogue → a product brief + the list of workflows to model. You
   approve the brief; approval queues each workflow for event modeling.
2. **Event modeling** (lightly interactive) — `/claude-factory:model`
   Drives `emc` to author and formally verify each workflow's slices. Follow the
   agent's lead; verification (`emc verify`) is the gate.
3. **Architecture** (interactive) — `/claude-factory:architect`
   For each verified slice, an *architecture triage* asks "does this force a new
   cross-cutting decision (an ADR)?" You confirm; most slices fast-pass, early
   ones set the baseline. Needed ADRs are drafted, cross-reviewed, and you decide.
4. **Design system** (interactive) — `/claude-factory:design`
   For each slice, a *design triage* asks "does this need UI components we don't
   have yet?" You confirm; missing components are built — reusable ones into the
   platform UI library, bespoke ones owned by the slice (ADR 0012).
5. **Development** (autonomous) — `/claude-factory:develop` or `/claude-factory:work`
   Enforced red-green-refactor per slice: write a failing test → independent test
   review → kernel runs it red → narrowest implementation → kernel runs it green →
   independent implementation review → lint → commit → open PR.
6. **Review** (PR gate — human) — `/claude-factory:review`
   The kernel polls the forge for CI + reviews + comments, triages new comments,
   and merges once green and approved. Approval is your call.

## The autonomous conductor

`/claude-factory:work` runs the whole loop, dispatching the next ready step across
all phases. It pauses for you at the interactive gates above (via a question) and
at the PR review gate; otherwise it runs unattended. Scope it with
`--phase <phase>`; single-step with `--once`. `/claude-factory:status` shows
per-phase WIP at any time.

## What flows automatically vs. what you decide

- **Automatic:** slice promotion (modeling → architecture → design → development →
  review), the TDD state machine, running checks, opening/polling/merging PRs.
- **Yours:** the discovery brief, architecture triage + ADR decisions, design
  triage + UX, and final PR approval. A triage agent only *recommends*; you decide.
