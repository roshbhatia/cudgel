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

    # Install ONNX models if not present (check for actual model file)
    MODEL_DIR="$HOME/.local/share/cudgel/models/all-MiniLM-L6-v2"
    MODEL_FILE="$MODEL_DIR/model.onnx"

    if [ ! -f "$MODEL_FILE" ]; then
      echo "Installing ONNX embedding model..."
      mkdir -p "$(dirname "$MODEL_DIR")"
      uv venv .venv
      source .venv/bin/activate
      uv pip install 'optimum[onnxruntime]'
      optimum-cli export onnx --model sentence-transformers/all-MiniLM-L6-v2 "$MODEL_DIR"
      deactivate
      echo "Model installed to $MODEL_DIR"
    fi
  '';

  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
