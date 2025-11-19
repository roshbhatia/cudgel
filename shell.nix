{
  pkgs ? import <nixpkgs> { },
}:

pkgs.mkShell {
  name = "cudgel-dev-shell";

  buildInputs = with pkgs; [
    cargo
    clippy
    git
    go-task
    # ollama
    openssl
    pkg-config
    postgresql17Packages.pgvector
    postgresql_17
    rust-analyzer
    rustc
    rustfmt
    uv
  ];

  shellHook = ''
    export RUST_BACKTRACE=1
    export CARGO_TARGET_DIR="target"
    export PATH="$HOME/.cargo/bin:$PATH"
  '';

  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
