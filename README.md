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

The compiled binary will be available in `target/release/export-cloudflare-dns`.

## Configuration

You will need to add your API credentials into a `.env` file:

1. Create a `.env` file in the same directory as the binary:

```bash
cp .env.example .env
```

2. Add your Cloudflare API key and email address to the `.env` file:

```bash
CLOUDFLARE_API_KEY=your_api_key_here
CLOUDFLARE_USER_EMAIL=your_email_here
```

## Usage

Simply run the binary:

```bash
./export-cloudflare-dns
```

If you want to use a custom environment file, you can specify it as an argument:

```bash
./export-cloudflare-dns custom.env
```

## Errors Explained

### Error: Unknown X-Auth-Key or X-Auth-Email

If you get an error message "Error: Unknown X-Auth-Key or X-Auth-Email", this means you haven't supplied a valid API key and email address in your environment file.

## License

[MIT](LICENSE)
