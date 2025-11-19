{
  pkgs ? import <nixpkgs> { },
}:

pkgs.mkShell {
  name = "cudgel-dev-shell";

  buildInputs = with pkgs; [
    rustc
    cargo
    rustfmt
    clippy
    rust-analyzer
    postgresql_17
    postgresql17Packages.pgvector
    pkg-config
    openssl
    git
    go-task
    uv
    ollama
  ];

  shellHook = ''
    export RUST_BACKTRACE=1
    export CARGO_TARGET_DIR="target"
    export PATH="$HOME/.cargo/bin:$PATH"
  '';

  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
