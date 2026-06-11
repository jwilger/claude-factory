---
name: Strict Linting
description: This skill should be used when configuring linters for a new project stack, understanding why the factory starts with the strictest linter config, or deciding whether a lint rule relaxation is justified. It explains the principle that strictness is the default and relaxation requires narrow scope and documented justification.
version: 1.0.0
---

Start with the strictest possible linting enabled. Relax only when forced, narrowly, with documented justification.

## The principle

Linters encode accumulated knowledge about bugs and bad patterns. Starting strict means you get that knowledge for free. Starting permissive and adding rules later means every rule addition breaks existing code — the cost compounds. Starting strict means occasional, justified relaxations — the cost is bounded.

"We'll add stricter rules later" never happens. Start strict.

## Rust: clippy configuration

Enable everything from the start. The minimum baseline:

```toml
# In Cargo.toml workspace or per-crate
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
all = "deny"
pedantic = "deny"
nursery = "warn"
```

See the billylevin.dev clippy-config post (referenced in `references/`) for the full recommended configuration including restriction lints worth enabling.

Relaxing a lint requires:
1. A specific, narrow `#[allow(clippy::rule_name)]` on the exact item (not a module or file-wide allow)
2. A comment explaining why the rule does not apply here

Never: `#![allow(clippy::all)]` or file-level allows for pedantic/style rules.

## TypeScript: ESLint configuration

Start with `@typescript-eslint/strict` + `@typescript-eslint/stylistic`. Enable `no-explicit-any` as error, not warning.

## Other stacks

Per-stack strict configurations are documented in `references/`. The principle is universal: strictest first, narrowest relaxation with justification.

See `references/` for stack-specific configurations.
