# Toolchain Setup

This document covers setting up the toolchains required to work on Claude-Factory and to use Claude-Factory in product projects.

## Quick start with Nix (recommended)

If you have Nix with flakes enabled:

```bash
# Clone the repo and enter the devshell
git clone https://github.com/jwilger/claude-factory.git
cd claude-factory
nix develop
```

The devshell provides: Rust nightly, cargo tooling (nextest, cargo-edit, cargo-watch), `elan` (Lean4 version manager), `lake`, Quint (via npx), jq, and SQLite.

First time in the devshell, you may need to install the Lean4 toolchain via elan:

```bash
elan install leanprover/lean4:stable
```

### Enabling flakes (if not already enabled)

Add to `~/.config/nix/nix.conf` or `/etc/nix/nix.conf`:

```
experimental-features = nix-command flakes
```

---

## Manual setup (macOS / Linux without Nix)

### 1. Rust nightly

Install rustup if you don't have it:

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

The `rust-toolchain.toml` in the repo root pins the nightly channel. After installing rustup, run:

```bash
rustup show  # installs the toolchain specified in rust-toolchain.toml automatically
```

Verify:

```bash
rustc --version  # should show a nightly date
cargo --version
cargo clippy --version
```

Install additional cargo tooling:

```bash
cargo install cargo-nextest --locked
cargo install cargo-edit
cargo install cargo-watch
```

### 2. Lean4 / lake (required for `emc verify`)

Install elan, the Lean version manager:

```bash
# macOS / Linux
curl https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh -sSf | sh
```

Then install the Lean toolchain:

```bash
elan install leanprover/lean4:stable
elan default leanprover/lean4:stable
```

Verify:

```bash
lean --version
lake --version
```

### 3. Quint (required for `emc verify`)

Requires Node.js 18+. Install Node.js via your package manager or nvm:

```bash
# macOS with Homebrew
brew install node

# Linux (via nvm)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install --lts
```

Then install Quint:

```bash
npm install -g @informalsystems/quint
```

Verify:

```bash
quint --version
```

### 4. emc (Event Model Compiler)

Once the crates.io publication is complete:

```bash
cargo install emc
```

Until then (git source):

```bash
cargo install --git https://github.com/jwilger/emc
```

Verify:

```bash
emc --version
```

### 5. SQLite (optional — only for building emc from source)

The cfk kernel no longer requires a system SQLite library: its event store is the
file-based `eventcore-fs` adapter (see ADR 0010), which persists events as
git-tracked JSONL files with no database. This section is retained only for users
who build `emc` from source, since `emc` still uses the `eventcore-sqlite` adapter.

**macOS**: included in the OS. No action needed.

**Linux**:

```bash
# Debian/Ubuntu
sudo apt install libsqlite3-dev

# Fedora/RHEL
sudo dnf install sqlite-devel

# Arch
sudo pacman -S sqlite
```

### 6. jq

```bash
# macOS
brew install jq

# Linux (Debian/Ubuntu)
sudo apt install jq

# Linux (Fedora)
sudo dnf install jq
```

---

## What each tool is used for

| Tool | Required for |
|---|---|
| Rust nightly + cargo | Building `cfk` (the factory kernel) |
| cargo-nextest | Running kernel tests |
| lean / lake | `emc verify` — formal proof checking of event models |
| quint | `emc verify` — behavioral verification of event models |
| emc | Event modeling in product projects |
| SQLite | Optional — only to build `emc` from source (emc uses eventcore-sqlite). The cfk kernel uses the file-based eventcore-fs adapter (ADR 0010). |
| jq | Shell scripts that process JSON (codex-runner.sh, bootstrap scripts) |

### What works without lean/lake and quint

If lean/lake and quint are not installed:
- All emc authoring tools work (`add_workflow`, `add_slice`, `check_project`, etc.)
- The **event-modeling gate** (`verify_project` → `WorkflowReadinessDeclared`) is not available
- Development slices will not be unblocked until verification is available

The bootstrap-emc.sh script will warn clearly about missing dependencies.

---

## Developing the kernel (build-on-demand dev loop)

The plugin's MCP server is launched by `plugins/claude-factory/scripts/bootstrap-cfk.sh`.
In this dev repo (where the `kernel/` sources are present) the bootstrap **builds `cfk` on
demand** — there is no manual `cargo build` → copy → commit → re-cache dance:

- On launch it hashes the kernel sources (`*.rs`, `Cargo.toml`, `Cargo.lock`). If the hash
  matches the last build, it reuses the cached binary at `plugins/claude-factory/.bin/cfk`
  (instant). If the sources changed, it rebuilds, caches, and records the new hash.
- It builds via `cargo` if on `PATH`, otherwise via `nix develop` automatically.
- The `.bin/` cache is **gitignored**; the binary is no longer committed to the repo.

**The loop:** edit the kernel → restart the claude-factory MCP server (restarting the Claude
Code session reloads the plugin) → the next launch rebuilds automatically. The first build of
a changed tree takes ~30s–a few minutes; launches with an unchanged tree are instant.

```bash
# Force a rebuild on next launch (e.g. after a dependency change cargo can't detect):
rm -f plugins/claude-factory/.bin/.kernel_src_hash

# Build/refresh the cache manually (optional — bootstrap does this for you):
bash plugins/claude-factory/scripts/bootstrap-cfk.sh </dev/null   # builds, then exits when no MCP peer
```

> The kernel-edit guardrail and statusline resolve the same binary (`.bin/cfk`, falling back
> to a prebuilt `bin/cfk`). When building the kernel directly (outside the factory loop), set
> `.claude-factory/LEASE_BYPASS` so edits to `kernel/**/src/**` aren't blocked by the lease
> guardrail.

---

## Product projects

Product repos that use Claude-Factory need the same toolchain as this repo, plus whatever their product stack requires. Recommended approach:

1. Copy or symlink this repo's `flake.nix` and `rust-toolchain.toml` into the product repo, extending with product-specific packages
2. Or follow the manual setup above and document any additional product-stack deps

Product projects that install the plugin **alongside the kernel sources** (e.g. this repo as a
local marketplace) get the build-on-demand loop described above. A standalone consumer install
without the `kernel/` tree needs a prebuilt `cfk` at `plugins/claude-factory/bin/cfk`; shipping
per-platform prebuilt binaries is a packaging task tracked for a later milestone (M8).
