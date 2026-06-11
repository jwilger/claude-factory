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
cargo install --git https://git.johnwilger.com/Slipstream/emc
```

Verify:

```bash
emc --version
```

### 5. SQLite (for the cfk kernel)

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
| SQLite | cfk-engine's event store (eventcore-sqlite) |
| jq | Shell scripts that process JSON (codex-runner.sh, bootstrap scripts) |

### What works without lean/lake and quint

If lean/lake and quint are not installed:
- All emc authoring tools work (`add_workflow`, `add_slice`, `check_project`, etc.)
- The **event-modeling gate** (`verify_project` → `WorkflowReadinessDeclared`) is not available
- Development slices will not be unblocked until verification is available

The bootstrap-emc.sh script will warn clearly about missing dependencies.

---

## Product projects

Product repos that use Claude-Factory need the same toolchain as this repo, plus whatever their product stack requires. Recommended approach:

1. Copy or symlink this repo's `flake.nix` and `rust-toolchain.toml` into the product repo, extending with product-specific packages
2. Or follow the manual setup above and document any additional product-stack deps

Product projects do **not** need to build `cfk` — the claude-factory plugin's bootstrap-cfk.sh handles that automatically.
