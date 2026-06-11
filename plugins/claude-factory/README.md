# claude-factory plugin

The conductor plugin for the Claude-Factory dark software factory.

## What it provides

- **Commands**: `/claude-factory:work` (main loop), `:init`, `:status`, `:discover`, `:model`, `:architect`, `:design`, `:develop`, `:review`
- **Agents**: discovery-partner, event-modeler, architect, adr-reviewer, test-writer, test-reviewer, implementer, implementation-reviewer, pr-shepherd-triage, researcher
- **Skills**: tdd-protocol, semantic-types, fcis-and-effects, railway-oriented-programming, event-modeling-method, event-sourcing-patterns, atomic-design, vertical-slice-architecture, strict-linting, factory-conductor
- **MCP server**: `cfk` — the deterministic factory kernel (Rust, built from `../../kernel/`)
- **Hooks**: SessionStart status display; PreToolUse lease guard

## Requirements

- `nix develop` (from repo root) or manual toolchain setup per `../../docs/SETUP.md`
- The `emc` plugin (for event modeling phase)

## Quick start

```bash
# In a product repo
/claude-factory:init
/claude-factory:discover
/claude-factory:work
```

See the [project ROADMAP](../../ROADMAP.md) for the full product plan.
