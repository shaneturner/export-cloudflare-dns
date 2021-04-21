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

First create a blank `.env` file in the base directory or copy the example file `.env.example` to `.env`

```BASH
cp .env.example .env
```

Then add the appropriate values for your API key and User email address in the new .env file

```BASH
CLOUDFLARE_API_KEY=[YOUR API KEY HERE]

CLOUDFLARE_USER_EMAIL=[YOUR USER/LOGIN EMAIL HERE]
```
Change the NULL values to the values supllied in your Cloudlfar account.

## Usage

```
npm run get-domains
```

## License

[MIT](LICENSE)
