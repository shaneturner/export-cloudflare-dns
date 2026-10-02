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
```

No test suite exists currently.

## Configuration

Credentials come from the environment or a `.env` file (see `.env.example`). Empty values and the `NULL` placeholder count as unset:
- `CLOUDFLARE_API_TOKEN` - API token, sent as `Authorization: Bearer` (preferred)
- `CLOUDFLARE_API_KEY` + `CLOUDFLARE_USER_EMAIL` - Global API key fallback, sent as `X-Auth-Key` / `X-Auth-Email`

## Architecture

Single-file application (`src/main.rs`) with this flow:

1. `main()` — parses `Cli` (clap derive), builds one shared client, fetches domains, exports each
2. `load_auth()` — loads the env file (default `.env` is optional; a custom path must exist), returns an `Auth` (token or key + email)
3. `create_client()` — builds the reqwest client with the auth headers for the given `Auth`
4. `get_domains()` — paginated `GET /zones` calls, returns all `Domain` structs
5. `export_dns()` — `GET /zones/{id}/dns_records/export` for one domain, writes BIND-format text to the output directory

API base: `https://api.cloudflare.com/client/v4/`

Structs: `CloudflareResponse`, `Domain`, `ResultInfo`, `CloudflareError` — all derive `Deserialize`. `result` and `result_info` are `Option` because Cloudflare error responses return `result: null` without `result_info`.

## Dependencies

clap (CLI args), reqwest (HTTP + JSON), tokio (async runtime), serde/serde_json (serialization), dotenvy (env file loading).
