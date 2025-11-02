{
  pkgs ? import <nixpkgs> { },
}:

pkgs.mkShell {
  name = "cudgel-dev-shell";

  buildInputs = with pkgs; [
    # Rust toolchain
    rustc
    cargo
    rustfmt
    clippy
    rust-analyzer

    # PostgreSQL 17 with pgvector (native, no Docker)
    postgresql_17
    postgresql17Packages.pgvector

    # Build dependencies
    pkg-config
    openssl
    git

    # Task automation
    go-task

    # Python tooling for ONNX model export
    uv

    # LLM for knowledge generation (User Story 3)
    ollama
  ];

  shellHook = ''
    export RUST_BACKTRACE=1
    export CARGO_TARGET_DIR="target"
    export PATH="$HOME/.cargo/bin:$PATH"
  '';

  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
