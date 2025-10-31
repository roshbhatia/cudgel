{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "cudgel-dev-shell";

  buildInputs = with pkgs; [
    # Rust toolchain
    rustc
    cargo
    rustfmt
    clippy
    rust-analyzer

    # PostgreSQL with pgvector
    postgresql_16
    pgvector

    # Development tools
    docker
    docker-compose

    # Task automation
    go-task

    # Other utilities
    git
    pkg-config
    openssl
  ];

  shellHook = ''
    echo "Cudgel Development Environment"
    echo ""
    echo "Quick start:"
    echo "  task build    - Build the project"
    echo "  task test     - Run tests"
    echo "  task refresh  - Rebuild and reinstall"
    echo ""

    # Set environment variables
    export RUST_BACKTRACE=1
    export CARGO_TARGET_DIR="target"

    # Add cargo bin to PATH
    export PATH="$HOME/.cargo/bin:$PATH"
  '';

  # Environment variables
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
