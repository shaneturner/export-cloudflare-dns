# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rust CLI tool that bulk-exports Cloudflare DNS records (BIND format) for all domains in an account. Authenticates via a Cloudflare API token (or legacy Global API key + email), paginates through all zones, and writes each domain's DNS records to `<output>/<domainname>.txt` (default `./domains`).

## Build & Run

```bash
cargo build --release          # Build release binary
cargo run                      # Run with default .env
cargo run -- custom.env        # Run with custom env file
cargo run -- -o backups        # Write to a different output directory
cargo run -- --help            # Show CLI usage
cargo run -- --setup           # Interactive setup wizard
```

No test suite exists currently.

## Configuration

Credentials are loaded from the first of: the `[ENV_FILE]` argument, `cloudflare-export.env` next to the executable (written by `--setup`), `.env` in the current directory, then plain environment variables. Empty values and the `NULL` placeholder count as unset:
- `CLOUDFLARE_API_TOKEN` - API token, sent as `Authorization: Bearer` (preferred)
- `CLOUDFLARE_API_KEY` + `CLOUDFLARE_USER_EMAIL` - Global API key fallback, sent as `X-Auth-Key` / `X-Auth-Email`

## Architecture

Single-file application (`src/main.rs`) with this flow:

1. `main()` — parses `Cli` (clap derive), resolves an `Auth` via `--setup`, `load_auth()`, or (when stdin is a terminal and no credentials exist) an offer to run setup, then calls `run()`
2. `load_auth()` — finds and loads the config file (see lookup order above), returns `Option<Auth>` (token or key + email)
3. `setup()` — wizard: prompts for a hidden API token, verifies it with `fetch_zones_page()`, saves it via `save_config()` (mode 0600 on Unix)
4. `run()` — builds one shared client, fetches domains, exports each
5. `fetch_zones_page()` / `get_domains()` — one `GET /zones` page (returns a user-facing error string) / paginates through all of them
6. `export_dns()` — `GET /zones/{id}/dns_records/export` for one domain, writes BIND-format text to the output directory
7. `create_client()` — builds the reqwest client with the auth headers for the given `Auth`

Use `exit()` (not `process::exit`) so `pause_if_own_console()` keeps a double-clicked Windows console window open before exiting.

API base: `https://api.cloudflare.com/client/v4/`

Structs: `CloudflareResponse`, `Domain`, `ResultInfo`, `CloudflareError` — all derive `Deserialize`. `result` and `result_info` are `Option` because Cloudflare error responses return `result: null` without `result_info`.

## Dependencies

clap (CLI args), rpassword (hidden token input), reqwest (HTTP + JSON), tokio (async runtime), serde/serde_json (serialization), dotenvy (env file loading).

## Git Conventions

- Never add `Co-Authored-By`, `Claude-Session`, "Generated with Claude Code" or any other AI/Claude attribution to commit messages, PR titles/descriptions, or other git metadata. `.claude/settings.json` disables Claude Code's automatic attribution.
- Use descriptive branch names that reflect the change (e.g. `feature/setup-wizard`, `fix/api-error-parsing`); don't include "claude" in branch names.
