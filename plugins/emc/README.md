# emc plugin

Packages the [emc (Event Model Compiler)](https://github.com/jwilger/emc) MCP server for use with Claude Code.

## What it provides

- **MCP server**: `emc mcp stdio` — 23 tools for authoring and formally verifying event models using Lean4 and Quint

## Requirements

- `cargo` (for `cargo install emc`)
- `lake` (Lean4) — required for `emc verify` (formal proof checking)
- `quint` — required for `emc verify` (behavioral verification)

See `../../docs/SETUP.md` for installation instructions.

## Bootstrap behavior

On first use, `scripts/bootstrap-emc.sh`:
1. Installs emc via `cargo install emc` (falls back to git source if not yet on crates.io)
2. Checks for `lake` and `quint` and warns if missing
3. Starts `emc mcp stdio`

Without `lake` and `quint`, event model authoring works but the formal verification gate (`verify_project` → `WorkflowReadinessDeclared`) is unavailable.

## emc tools

See [emc documentation](https://github.com/jwilger/emc) for the full tool reference.

Key tools: `init_project`, `add_workflow`, `add_slice`, `connect_workflow`, `check_project`, `verify_project`, `review_gate`, `record_clean_review`.
