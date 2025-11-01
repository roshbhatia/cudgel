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

    postgresql_18
    postgresql18Packages.pgvector

    docker
    docker-compose

    go-task

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

    export RUST_BACKTRACE=1
    export CARGO_TARGET_DIR="target"
    export PATH="$HOME/.cargo/bin:$PATH"
  '';

  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
