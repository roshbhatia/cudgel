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

    # Development tools
    docker
    docker-compose
    postgresql_16

    # Task automation
    go-task

    # Git hooks
    pre-commit

    # Other utilities
    git
    gnumake
    pkg-config
    openssl
  ];

  shellHook = ''
    echo "🔨 Cudgel Development Environment"
    echo ""
    echo "Available commands:"
    echo "  cargo build           - Build the project"
    echo "  cargo test            - Run tests"
    echo "  cargo clippy          - Run linter"
    echo "  cargo fmt             - Format code"
    echo "  task                  - Run tasks (see Taskfile.yml)"
    echo "  pre-commit install    - Install git hooks"
    echo ""
    echo "Quick start:"
    echo "  1. task install-hooks  - Setup git hooks"
    echo "  2. task build          - Build project"
    echo "  3. task test           - Run tests"
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
