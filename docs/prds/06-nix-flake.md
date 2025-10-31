# PRD: Nix Flake Distribution

## Overview
Package Cudgel as a Nix flake for easy installation and reproducible builds across NixOS and other systems with Nix.

## Goals
1. Enable `nix run github:roshbhatia/cudgel` to work instantly
2. Provide development shell with all dependencies
3. Support NixOS module for running as a service
4. Ensure reproducible builds

## Non-Goals
- Supporting legacy Nix (pre-flakes)
- Replacing Cargo/Homebrew installations
- Packaging for nixpkgs (separate future effort)

## Success Metrics
- Installation time: <2 minutes (first time with cache)
- Zero manual dependency installation
- Works on NixOS, macOS, and Linux with Nix
- Development shell includes all tools

## Detailed Requirements

### 1. Flake Structure

**File**: `flake.nix`

```nix
{
  description = "Code indexing tool with semantic search";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
      in
      {
        packages.default = pkgs.callPackage ./nix/package.nix { };

        devShells.default = pkgs.callPackage ./nix/shell.nix { };

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/cudgel";
        };
      }
    ) // {
      # NixOS module
      nixosModules.default = import ./nix/module.nix;
    };
}
```

**Acceptance Criteria:**
- [ ] `nix flake check` passes
- [ ] All outputs are defined
- [ ] Lock file tracks dependencies
- [ ] Compatible with Nix 2.11+

### 2. Package Definition

**File**: `nix/package.nix`

**Requirements:**
- Build from source using Cargo
- Include PostgreSQL 17 with pgvector
- Install all necessary dependencies
- Generate shell completions
- Include man pages (if they exist)

**Build Process:**
```nix
{ lib, rustPlatform, postgresql_17, pgvector, pkg-config, openssl }:

rustPlatform.buildRustPackage rec {
  pname = "cudgel";
  version = "0.1.0";

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ openssl postgresql_17 pgvector ];

  # Tests require PostgreSQL
  doCheck = false;
  checkInputs = [ postgresql_17 ];

  preCheck = ''
    export PGDATA=$TMPDIR/postgres
    initdb -D $PGDATA
    pg_ctl -D $PGDATA start
    createdb cudgel
  '';

  postCheck = ''
    pg_ctl -D $PGDATA stop
  '';

  meta = with lib; {
    description = "Code indexing tool with semantic search";
    homepage = "https://github.com/roshbhatia/cudgel";
    license = licenses.mit;
    maintainers = with maintainers; [ ];
  };
}
```

**Acceptance Criteria:**
- [ ] Builds successfully with Nix
- [ ] All Rust dependencies resolved
- [ ] PostgreSQL and pgvector included
- [ ] Binary works after installation
- [ ] Tests run in nix build (optional)

### 3. Development Shell

**File**: `nix/shell.nix`

**Requirements:**
- All development tools available
- PostgreSQL with pgvector
- Rust toolchain (stable)
- Task (go-task)
- Pre-commit hooks

**Implementation:**
```nix
{ pkgs }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    # Rust toolchain
    (rust-bin.stable.latest.default.override {
      extensions = [ "rust-src" "rust-analyzer" ];
    })

    # PostgreSQL with pgvector
    postgresql_17
    pgvector

    # Build dependencies
    pkg-config
    openssl

    # Development tools
    go-task
    pre-commit
    git

    # Optional: Docker for Temporal (if needed)
    docker
    docker-compose
  ];

  shellHook = ''
    echo "Cudgel development environment"
    echo "PostgreSQL 17 with pgvector available"
    echo ""
    echo "Quick start:"
    echo "  task setup    - Build and set up environment"
    echo "  task build    - Build the project"
    echo "  task test     - Run tests"

    export RUST_BACKTRACE=1
    export PATH="$HOME/.cargo/bin:$PATH"
  '';
}
```

**Acceptance Criteria:**
- [ ] `nix develop` enters shell successfully
- [ ] All commands available (cargo, task, etc.)
- [ ] PostgreSQL binaries in PATH
- [ ] Environment variables set correctly
- [ ] Shell hook provides helpful info

### 4. NixOS Module

**File**: `nix/module.nix`

**Purpose**: Run Cudgel as a systemd service on NixOS

**Configuration:**
```nix
{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.cudgel;
in
{
  options.services.cudgel = {
    enable = mkEnableOption "Cudgel code indexing service";

    package = mkOption {
      type = types.package;
      default = pkgs.cudgel;
      description = "Cudgel package to use";
    };

    port = mkOption {
      type = types.port;
      default = 54321;
      description = "PostgreSQL port";
    };

    dataDir = mkOption {
      type = types.path;
      default = "/var/lib/cudgel";
      description = "Data directory for PostgreSQL";
    };

    user = mkOption {
      type = types.str;
      default = "cudgel";
      description = "User to run service as";
    };
  };

  config = mkIf cfg.enable {
    systemd.services.cudgel-postgres = {
      description = "PostgreSQL for Cudgel";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      serviceConfig = {
        Type = "forking";
        User = cfg.user;
        ExecStart = "${pkgs.postgresql_17}/bin/pg_ctl start -D ${cfg.dataDir}";
        ExecStop = "${pkgs.postgresql_17}/bin/pg_ctl stop -D ${cfg.dataDir}";
        Restart = "on-failure";
      };

      preStart = ''
        if [ ! -d ${cfg.dataDir} ]; then
          ${pkgs.postgresql_17}/bin/initdb -D ${cfg.dataDir}
        fi
      '';
    };

    users.users.${cfg.user} = mkIf (cfg.user == "cudgel") {
      isSystemUser = true;
      description = "Cudgel service user";
      group = cfg.user;
      home = cfg.dataDir;
    };

    users.groups.${cfg.user} = mkIf (cfg.user == "cudgel") {};
  };
}
```

**Acceptance Criteria:**
- [ ] Service starts automatically on boot
- [ ] PostgreSQL data persists across reboots
- [ ] Proper user/group management
- [ ] Logs accessible via journalctl
- [ ] Can be configured via NixOS config

### 5. Installation Methods

#### Nix Run (Temporary)
```bash
nix run github:roshbhatia/cudgel
```

#### Nix Profile Install
```bash
nix profile install github:roshbhatia/cudgel
```

#### NixOS Configuration
```nix
{
  inputs.cudgel.url = "github:roshbhatia/cudgel";

  outputs = { self, nixpkgs, cudgel }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      modules = [
        cudgel.nixosModules.default
        {
          services.cudgel.enable = true;
        }
      ];
    };
  };
}
```

#### Development Shell
```bash
nix develop github:roshbhatia/cudgel
```

**Acceptance Criteria:**
- [ ] All installation methods work
- [ ] Documentation covers each method
- [ ] Examples provided
- [ ] Common issues documented

### 6. Binary Cache

**Optional**: Set up Cachix for pre-built binaries

**Configuration:**
```yaml
# .github/workflows/cachix.yml
name: Cachix
on: [push]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: cachix/install-nix-action@v20
      - uses: cachix/cachix-action@v12
        with:
          name: cudgel
          authToken: '${{ secrets.CACHIX_AUTH_TOKEN }}'
      - run: nix build
```

**Acceptance Criteria:**
- [ ] Cache set up (if implemented)
- [ ] Builds pushed to cache
- [ ] Users can use cache
- [ ] Cache configuration documented

## Implementation Plan

### Phase 1: Basic Flake (Week 1)
1. Create `flake.nix` with minimal outputs
2. Implement package definition
3. Test build on NixOS and macOS
4. Fix any build issues

### Phase 2: Development Shell (Week 2)
1. Create comprehensive dev shell
2. Include all tools and dependencies
3. Add helpful shell hook
4. Test on multiple systems

### Phase 3: NixOS Module (Week 3)
1. Implement systemd service
2. Add configuration options
3. Test on NixOS
4. Document module usage

### Phase 4: Documentation (Week 4)
1. Write Nix installation guide
2. Add examples for each use case
3. Document troubleshooting
4. Create NixOS configuration examples

### Phase 5: Testing & Polish (Week 5)
1. Test on NixOS, macOS, Linux
2. Verify all installation methods
3. Set up binary cache (optional)
4. Address feedback

## Dependencies
- Nix 2.11+ with flakes enabled
- GitHub repository
- (Optional) Cachix account for binary cache

## Risks & Mitigation

**Risk**: PostgreSQL setup differs on NixOS vs other systems
**Mitigation**: Test on multiple platforms, document platform-specific setup

**Risk**: Flake lock file becomes outdated
**Mitigation**: Dependabot for Nix inputs, regular updates

**Risk**: Breaking changes in nixpkgs
**Mitigation**: Pin nixpkgs version, test before updating

## Open Questions
- Should we include Temporal in Nix package?
- Do we need separate outputs for minimal vs full installation?
- Should we submit to nixpkgs after stabilization?

## Testing Plan

### Build Tests
- [ ] Builds on Linux (x86_64)
- [ ] Builds on macOS (x86_64, aarch64)
- [ ] Development shell works
- [ ] All dependencies available

### Integration Tests
- [ ] `nix run` works
- [ ] `nix profile install` works
- [ ] NixOS module works
- [ ] Binary runs correctly after installation

### Cross-Platform Tests
- [ ] NixOS system
- [ ] macOS with Nix
- [ ] Ubuntu with Nix
- [ ] Home Manager integration

## References
- [Nix Flakes Documentation](https://nixos.wiki/wiki/Flakes)
- [rust-overlay](https://github.com/oxalica/rust-overlay)
- [Nix Rust Guide](https://nix.dev/tutorials/building-and-running-c-programs)
- [NixOS Module System](https://nixos.org/manual/nixos/stable/index.html#sec-writing-modules)
