# Bulk export Cloudflare DNS records for all domains

Exports DNS records for each domain on a Cloudflare account.
Domain DNS records are created as individual domainname.txt files and will be placed in a "domains" subdirectory.

## Installation

Fastest way is:

```
npx degit shaneturner/export-cloudflare-dns#main YOUR-PROJECT-NAME
cd YOUR-PROJECT-NAME
npm install
```

## Configuration

You will need to add your API credentials into a .env file

```
cp .env.example .env
```

add the appropriate values for your API key and User email address in the .env file

## Usage

```
npm run get-domains
```

## License

[MIT](LICENSE)
