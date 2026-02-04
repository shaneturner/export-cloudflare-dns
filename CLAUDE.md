# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rust CLI tool that bulk-exports Cloudflare DNS records (BIND format) for all domains in an account. Authenticates via Cloudflare API key + email, paginates through all zones, and writes each domain's DNS records to `./domains/<domainname>.txt`.

## Build & Run

```bash
cargo build --release          # Build release binary
cargo run                      # Run with default .env
cargo run -- custom.env        # Run with custom env file
```

No test suite exists currently.

## Configuration

Requires a `.env` file (see `.env.example`):
- `CLOUDFLARE_API_KEY` - Cloudflare API key
- `CLOUDFLARE_USER_EMAIL` - Cloudflare account email

## Architecture

Single-file application (`src/main.rs`, ~370 lines) with this flow:

1. `main()` — orchestrates: load env → validate → fetch domains → export each
2. `check_environment()` — loads `.env` (or custom path from argv), validates credentials exist and aren't placeholders
3. `get_domains()` — paginated `GET /zones` calls, returns all `Domain` structs
4. `export_dns()` — `GET /zones/{id}/dns_records/export` for one domain, writes BIND-format text to `./domains/`
5. `create_client()` — builds reqwest client with `X-Auth-Key` and `X-Auth-Email` headers

API base: `https://api.cloudflare.com/client/v4/`

Structs: `CloudflareResponse`, `Domain`, `ResultInfo`, `CloudflareError` — all derive `Deserialize` for serde JSON parsing.

## Dependencies

reqwest (HTTP + JSON), tokio (async runtime), serde/serde_json (serialization), dotenv (env file loading).
