# Bulk Export Cloudflare DNS Records

Exports DNS records for each domain on a Cloudflare account. Domain DNS records are created as individual domainname.txt files and will be placed in a "domains" subdirectory.

This is a Rust implementation of the [original Node.js tool](https://github.com/shaneturner/export-cloudflare-dns), providing better performance and cross-platform support without requiring Node.js or npm.

## Installation

### Using Pre-built Binaries (Recommended)

Download the latest pre-built binary for your platform from the [releases page](https://github.com/shaneturner/cloudflare-dns-exporter/releases).

### Building from Source

If you prefer to build from source, you'll need Rust installed:

1. [Install Rust](https://www.rust-lang.org/tools/install)
2. Clone this repositorygit push -u origin main

```bash
cargo build --release
```

The compiled binary will be available in `target/release/cloudflare-dns-exporter`.

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
./cloudflare-dns-exporter
```

If you want to use a custom environment file, you can specify it as an argument:

```bash
./cloudflare-dns-exporter custom.env
```

## Errors Explained

### Error: Unknown X-Auth-Key or X-Auth-Email

If you get an error message "Error: Unknown X-Auth-Key or X-Auth-Email", this means you haven't supplied a valid API key and email address in your environment file.

## License

[MIT](LICENSE)
