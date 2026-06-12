---
name: Strict Linting
description: This skill should be used when configuring linters for a new project stack, understanding why the factory starts with the strictest linter config, or deciding whether a lint rule relaxation is justified. It explains the principle that strictness is the default and relaxation requires narrow scope and documented justification.
version: 1.1.0
---

Start with the strictest possible linting enabled — using an **allowlist model**. Relax only when forced, narrowly, with documented justification.

## The principle

Linters encode accumulated knowledge about bugs and bad patterns. Starting strict means you get that knowledge for free. Starting permissive and adding rules later means every rule addition breaks existing code — the cost compounds. Starting strict means occasional, justified relaxations — the cost is bounded.

"We'll add stricter rules later" never happens. Start strict.

## Rust: clippy configuration (allowlist model)

Enable all lint groups at `priority = -1` so per-lint overrides take precedence. The minimum baseline:

```toml
# In Cargo.toml [workspace.lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "deny", priority = -1 }
restriction = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
# Required when enabling the restriction group
blanket_clippy_restriction_lints = "allow"

# Enforce #[expect] over #[allow] — stale suppressions become compile errors
allow_attributes = "deny"
allow_attributes_without_reason = "deny"
```

After enabling restriction + nursery, **triage every warning** with one of three decisions:
1. **Fix the code** — preferred; removes the violation entirely.
2. **Globally allow in Cargo.toml** — for lints that contradict project style (e.g. `implicit_return`, `question_mark_used`). Add a one-line rationale comment.
3. **`#[expect]` at the specific site** — for genuine, rare exceptions. Requires a `reason` string.

### Handling exceptions

Always use `#[expect]` instead of `#[allow]`:

```rust
// Correct
#[expect(clippy::too_many_lines, reason = "exhaustive match over FactoryEvent variants; each arm is a simple projection step")]
pub fn apply_event(...) { ... }

// Wrong — allow_attributes = "deny" will reject this
#[allow(clippy::too_many_lines)]
pub fn apply_event(...) { ... }
```

`#[expect]` errors at compile time when the lint no longer fires — keeping suppressions honest.

Never: `#![allow(...)]` or module-level allows. Never `#[allow]` without a reason.

### Panic-family lints

`unwrap_used`, `expect_used`, `panic`, `indexing_slicing` etc. start at `"warn"` (from the restriction group). Ratchet to `"deny"` once all violations are removed. Test code may retain `#[expect(clippy::unwrap_used, reason = "...")]` where panicking on bad test setup is intentional.

## TypeScript: ESLint configuration

Start with `@typescript-eslint/strict` + `@typescript-eslint/stylistic`. Enable `no-explicit-any` as error, not warning.

## Other stacks

Per-stack strict configurations are documented in `references/`. The principle is universal: strictest first, allowlist over denylist, narrowest relaxation with justification.

See `references/` for stack-specific configurations.
