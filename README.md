# Bulk Export Cloudflare DNS Records

Exports DNS records for each domain on a Cloudflare account. Domain DNS records are created as individual domainname.txt files and will be placed in a "domains" subdirectory.

This is a Rust implementation of the [original Node.js tool](https://github.com/shaneturner/export-cloudflare-dns-js), providing better performance and cross-platform support without requiring Node.js or npm.

## Installation

### Using Pre-built Binaries (Recommended)

Download the latest binary for your platform from the [releases page](https://github.com/shaneturner/export-cloudflare-dns/releases).

| Platform | Binary |
|----------|--------|
| Linux (x86_64) | `cloudflare-dns-exporter-linux-amd64` |
| Linux (ARM64) | `cloudflare-dns-exporter-linux-arm64` |
| macOS (Intel) | `cloudflare-dns-exporter-macos-amd64` |
| macOS (Apple Silicon) | `cloudflare-dns-exporter-macos-arm64` |
| Windows (x86_64) | `cloudflare-dns-exporter-windows-amd64.exe` |

**Linux / macOS:**

```bash
chmod +x cloudflare-dns-exporter-*
./cloudflare-dns-exporter-linux-amd64   # or the appropriate binary for your platform
```

**Windows:**

```powershell
.\cloudflare-dns-exporter-windows-amd64.exe
```

### Building from Source

If you prefer to build from source, you'll need Rust installed:

1. [Install Rust](https://www.rust-lang.org/tools/install)
2. Clone this repository
3. Build the project

```bash
cargo build --release
```

The compiled binary will be available in `target/release/cloudflare-dns-exporter`.

## Configuration

Create a `.env` file in the directory you run the binary from:

```bash
cp .env.example .env
```

Then add **one** of the following:

**API token (recommended)** — create one in the Cloudflare dashboard under *My Profile → API Tokens* with `Zone:Read` and `DNS:Read` permissions:

```bash
CLOUDFLARE_API_TOKEN=your_api_token_here
```

**Global API key** — used only if no token is set:

```bash
CLOUDFLARE_API_KEY=your_api_key_here
CLOUDFLARE_USER_EMAIL=your_email_here
```

These can also be set as regular environment variables instead of using a `.env` file.

## Usage

```text
Usage: cloudflare-dns-exporter [OPTIONS] [ENV_FILE]

Arguments:
  [ENV_FILE]  Env file to load credentials from [default: .env, if present]

Options:
  -o, --output <OUTPUT>  Directory to write the <domain>.txt files to [default: domains]
  -h, --help             Print help
  -V, --version          Print version
```

Examples:

```bash
./cloudflare-dns-exporter                      # uses .env, writes to ./domains
./cloudflare-dns-exporter custom.env           # uses a custom env file
./cloudflare-dns-exporter -o backups/dns       # writes to a different directory
```

## Errors Explained

### Error: Unknown X-Auth-Key or X-Auth-Email

If you get an error message "Error: Unknown X-Auth-Key or X-Auth-Email", this means you haven't supplied a valid API key and email address in your environment file.

### Error: Invalid API Token

The `CLOUDFLARE_API_TOKEN` is wrong, expired, or revoked. Create a new token with `Zone:Read` and `DNS:Read` permissions.

## License

[MIT](LICENSE)
