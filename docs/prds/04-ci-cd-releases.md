# PRD: CI/CD & Automated Releases

## Overview
Implement comprehensive CI/CD pipeline with automated testing, building, and releasing for multiple platforms.

## Goals
1. Automate testing on every PR and commit
2. Build release binaries for Linux, macOS, and Windows
3. Automatically generate changelogs and release notes
4. Publish to package managers (Homebrew, crates.io)

## Non-Goals
- App store distribution (macOS App Store, Windows Store)
- Binary signing/notarization (for now)
- Docker image publishing

## Success Metrics
- 100% of PRs run through CI before merge
- Release process time: <30 minutes from tag to published
- Zero manual steps in release process
- All releases include changelog

## Detailed Requirements

### 1. CI Workflow

**File**: `.github/workflows/ci.yml`

**Triggers:**
- Push to `main` branch
- Pull requests to `main`
- Scheduled (daily, to catch dependency issues)

**Jobs:**

#### Job: Test
**Runs on**: Ubuntu, macOS, Windows
**Steps:**
1. Checkout code
2. Install Rust toolchain (stable)
3. Cache dependencies
4. Start PostgreSQL (using actions/setup-postgres or Docker)
5. Run `cargo test`
6. Upload test results

**Acceptance Criteria:**
- [ ] Tests run on all 3 platforms
- [ ] PostgreSQL starts successfully in CI
- [ ] Test failures cause workflow to fail
- [ ] Test results visible in PR checks

#### Job: Lint
**Runs on**: Ubuntu
**Steps:**
1. Checkout code
2. Install Rust toolchain
3. Run `cargo fmt --check`
4. Run `cargo clippy -- -D warnings`

**Acceptance Criteria:**
- [ ] Fails if code not formatted
- [ ] Fails on any clippy warnings
- [ ] Fast (<2 minutes)

#### Job: Security Audit
**Runs on**: Ubuntu
**Steps:**
1. Checkout code
2. Install cargo-audit
3. Run `cargo audit`

**Acceptance Criteria:**
- [ ] Detects vulnerable dependencies
- [ ] Runs on schedule (daily)
- [ ] Creates issue if vulnerabilities found

#### Job: Coverage
**Runs on**: Ubuntu
**Steps:**
1. Checkout code
2. Install tarpaulin
3. Generate coverage report
4. Upload to Codecov

**Acceptance Criteria:**
- [ ] Coverage report visible in PR
- [ ] Tracks coverage over time
- [ ] Fails if coverage drops >5%

### 2. Release Workflow

**File**: `.github/workflows/release.yml`

**Triggers:**
- Tag matching `v*` (e.g., `v0.2.0`)

**Jobs:**

#### Job: Build Binaries
**Matrix**: Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64)

**Steps:**
1. Checkout code at tag
2. Install Rust toolchain
3. Build release binary (`cargo build --release`)
4. Strip and compress binary
5. Generate checksums (SHA256)
6. Upload artifacts

**Acceptance Criteria:**
- [ ] Produces binaries for all platforms
- [ ] Binaries are optimized (LTO enabled)
- [ ] Binaries work on target platform
- [ ] Checksums generated for verification

#### Job: Create Release
**Depends on**: Build Binaries
**Steps:**
1. Download all artifacts
2. Generate changelog from commits
3. Create GitHub Release
4. Upload binaries and checksums
5. Mark as pre-release if version contains `-`

**Changelog Format:**
```markdown
## What's Changed
### Features
- Add new feature X (#123)
- Improve feature Y (#124)

### Bug Fixes
- Fix crash in parser (#125)
- Resolve database connection issue (#126)

### Documentation
- Update README with examples (#127)

**Full Changelog**: https://github.com/user/repo/compare/v0.1.0...v0.2.0
```

**Acceptance Criteria:**
- [ ] GitHub Release created automatically
- [ ] Changelog includes all commits since last tag
- [ ] Binaries attached to release
- [ ] Release notes are readable and categorized

#### Job: Publish to crates.io
**Depends on**: Create Release
**Steps:**
1. Verify version matches tag
2. Run `cargo publish`

**Acceptance Criteria:**
- [ ] Published to crates.io automatically
- [ ] Only runs on non-pre-release tags
- [ ] Uses GitHub secret for API token
- [ ] Fails gracefully if version already published

### 3. Homebrew Formula Update

**Repository**: `roshbhatia/homebrew-cudgel`

**Automation:**
- GitHub Action triggered by new release
- Updates formula with new version
- Updates SHA256 checksums
- Opens PR to homebrew-cudgel repo

**Formula Template:**
```ruby
class Cudgel < Formula
  desc "Code indexing tool with semantic search"
  homepage "https://github.com/roshbhatia/cudgel"
  url "https://github.com/roshbhatia/cudgel/archive/v0.2.0.tar.gz"
  sha256 "..."

  depends_on "rust" => :build
  depends_on "postgresql@17"

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    system "#{bin}/cudgel", "--version"
  end
end
```

**Acceptance Criteria:**
- [ ] Formula automatically updated on release
- [ ] Users can install with `brew install roshbhatia/cudgel/cudgel`
- [ ] Formula includes all dependencies
- [ ] Test verifies binary works

### 4. Semantic Versioning & Changelog

**Tool**: [conventional-changelog](https://github.com/conventional-changelog/conventional-changelog)

**Commit Message Format:**
```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature (minor version bump)
- `fix`: Bug fix (patch version bump)
- `docs`: Documentation only
- `refactor`: Code change that neither fixes nor adds feature
- `perf`: Performance improvement
- `test`: Adding tests
- `chore`: Maintenance

**Examples:**
```
feat(parser): add support for TypeScript
fix(database): resolve connection pool exhaustion
docs(readme): update installation instructions
```

**Acceptance Criteria:**
- [ ] All commits follow conventional format
- [ ] PRs have conventional title
- [ ] Changelog auto-generated from commits
- [ ] Version bumped according to semver

### 5. Quality Gates

**Pre-merge Requirements:**
- [ ] All CI jobs pass
- [ ] Code review approved
- [ ] No merge conflicts
- [ ] Branch up-to-date with main

**Branch Protection Rules:**
- Require PR before merging
- Require status checks to pass
- Require code review
- Restrict who can push to main

**Acceptance Criteria:**
- [ ] Cannot merge if tests fail
- [ ] Cannot merge without review
- [ ] Status checks visible in PR
- [ ] Clear feedback on failures

## Implementation Plan

### Phase 1: Basic CI (Week 1)
1. Create `.github/workflows/ci.yml`
2. Set up test job for Ubuntu
3. Configure PostgreSQL in CI
4. Add lint job
5. Enable branch protection

### Phase 2: Multi-Platform CI (Week 2)
1. Extend tests to macOS and Windows
2. Set up matrix builds
3. Add security audit job
4. Configure coverage reporting

### Phase 3: Release Automation (Week 3)
1. Create `.github/workflows/release.yml`
2. Build binaries for all platforms
3. Auto-generate changelogs
4. Upload release artifacts

### Phase 4: Package Managers (Week 4)
1. Set up crates.io publishing
2. Create Homebrew tap repository
3. Auto-update Homebrew formula
4. Document installation methods

### Phase 5: Polish (Week 5)
1. Optimize CI caching
2. Improve error messages
3. Add workflow badges to README
4. Write contribution guide for releases

## Dependencies
- GitHub Actions enabled
- Homebrew tap repository created
- crates.io account and API token
- Codecov account (for coverage)

## Risks & Mitigation

**Risk**: CI costs money for private repos
**Mitigation**: Use public repo, optimize caching, use self-hosted runners if needed

**Risk**: Cross-compilation failures
**Mitigation**: Test builds locally first, use proven cross-compilation actions

**Risk**: Homebrew formula breaks
**Mitigation**: Automated tests in formula, manual verification before release

## Open Questions
- Should we support AUR (Arch Linux)?
- Do we need ARM64 Windows builds?
- Should we publish Docker images?

## References
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [cargo-dist](https://github.com/axodotdev/cargo-dist) - Alternative release automation
- [Conventional Commits](https://www.conventionalcommits.org/)
