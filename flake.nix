{
  description = "Claude-Factory: dark software factory devshell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, fenix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        # Rust nightly toolchain with all components needed for the kernel.
        # Uses fenix's nightly channel, which tracks rust-toolchain.toml's channel = "nightly".
        # Override the complete set so clippy, rustfmt, rust-src, and rust-analyzer are included.
        rustToolchain = fenix.packages.${system}.combine [
          fenix.packages.${system}.latest.rustc
          fenix.packages.${system}.latest.cargo
          fenix.packages.${system}.latest.clippy
          fenix.packages.${system}.latest.rustfmt
          fenix.packages.${system}.latest.rust-src
          fenix.packages.${system}.latest.rust-analyzer
        ];

        # Quint via npm — build a derivation wrapping npx
        quint = pkgs.writeShellScriptBin "quint" ''
          exec ${pkgs.nodejs}/bin/npx --yes @informalsystems/quint@latest "$@"
        '';

      in {
        devShells.default = pkgs.mkShell {
          name = "claude-factory";

          buildInputs = [
            # Rust nightly (cfk kernel)
            rustToolchain

            # Cargo tooling
            pkgs.cargo-nextest      # test runner (nextest format)
            pkgs.cargo-edit         # cargo add/rm
            pkgs.cargo-watch        # file-watching test runner

            # Lean4 for emc verification (via elan — the Lean version manager)
            pkgs.elan               # provides `lake` and `lean`

            # Quint for emc behavioral verification
            quint
            pkgs.nodejs             # quint runtime dependency

            # General tooling
            pkgs.jq
            pkgs.git
            pkgs.openssl
            pkgs.pkg-config

            # SQLite (for cfk-engine's eventcore-sqlite)
            pkgs.sqlite
          ];

          shellHook = ''
            echo ""
            echo "  Claude-Factory devshell"
            echo "  ─────────────────────────────────────────────"
            echo "  Rust:       $(rustc --version 2>/dev/null || echo 'not found')"
            echo "  Cargo:      $(cargo --version 2>/dev/null || echo 'not found')"
            echo "  lake:       $(lake --version 2>/dev/null || echo 'not found (elan may need: elan install leanprover/lean4:stable)')"
            echo "  quint:      $(quint --version 2>/dev/null | head -1 || echo 'available via npx on first use')"
            echo "  jq:         $(jq --version 2>/dev/null || echo 'not found')"
            echo ""
            git config core.hooksPath .githooks 2>/dev/null || true
            echo "  Run 'cargo build' in kernel/ to build cfk."
            echo "  See docs/SETUP.md for toolchain details and non-Nix setup."
            echo ""
          '';

          # Environment variables
          RUST_BACKTRACE = "1";
          RUST_LOG = "info";
        };
      });
}
