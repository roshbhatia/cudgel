# Cudgel Setup

## Quick Start

```bash
# One command setup: builds, installs, starts PostgreSQL, initializes database
task setup

# Index a repository
cudgel index .

# Query indexed code
cudgel query "parser" --limit 5

# View graph relationships
cudgel graph DatabaseConfig --depth 2
```

## Architecture

- **PostgreSQL**: Native process on port **54321** (non-standard to avoid conflicts)
- **Data location**: `~/.local/share/cudgel/postgres` (XDG compliant)
- **Dependencies**: PostgreSQL 17 with pgvector (via Homebrew)
- **No Docker required** for basic usage (optional for Temporal)

## Database Management

```bash
task db-start   # Start PostgreSQL on port 54321
task db-stop    # Stop PostgreSQL
task db-status  # Check if running
task db-clean   # Remove all data
```

## Configuration

All settings are hardcoded in `src/config.rs`:
- PostgreSQL: `localhost:54321`
- Database: `cudgel`
- User: Your system username
- Temporal: `localhost:7234` (if using scheduling)

No environment variables or config files needed!

## Requirements

```bash
# Install PostgreSQL 17 (includes pgvector support)
brew install postgresql@17

# Or use Nix shell
nix-shell
```

## Notes

- PostgreSQL data persists across restarts in `~/.local/share/cudgel/postgres`
- Logs available at `~/.local/share/cudgel/postgres.log`
- Custom ports avoid conflicts with system PostgreSQL
- Native processes are faster and more reliable than Docker for local development
