// "use strict";

const axios = require('axios');
const fs = require('fs');
const { exit } = require('process');

const args = process.argv.slice(2)

const CLOUDFLARE_ENDPOINT = 'https://api.cloudflare.com/client/v4/';
let domainList = [];

checkEnvironment();

// Fetch data from Cloudflare
console.info('Getting List of domains from Cloudflare');
console.info('=======================================\n');

// Get domain names from Cloudflare
getDomains();

function getDomains(getPage) {
  axios
    .get('zones', {
      baseURL: CLOUDFLARE_ENDPOINT,
      headers: {
        'X-Auth-Email': process.env.CLOUDFLARE_USER_EMAIL,
        'X-Auth-Key': process.env.CLOUDFLARE_API_KEY,
        'Content-Type': 'application/json',
      },
      params: {
        page: getPage ? getPage : 1,
      },
    })
    .then(function (response, domainList) {
      // Process Domain list
      if (response.data.success) {
        const pageInfo = response.data.result_info;

        console.log('Fetching batch of ' + pageInfo.count + ' DNS records ...');

        domainList = addDomainsToList(response.data);

        // Check for more pages in domain name list
        if (pageInfo.page < pageInfo.total_pages) {
          getDomains(pageInfo.page + 1);
        } else {
          console.log('Fetchied ' + pageInfo.total_count + ' domains.');
          return domainList;
        }
      } else {
        console.log(response.errors.message);
      }
    })
    .then(function (domains) {
      // Check to see if domains list is returned
      if (domains) {
        // Export Domain Records
        console.log('Writing domain DNS files');
        domains.forEach((domain) => {
          exportDNS(domain);
        });
        console.log(
          'Domain DNS records complete. Please check the /domains direcotry for your files'
        );
      }
    })
    .catch(function (error) {
      // ERROR HANDLING
      error.response.data.errors.forEach((error) => {
        console.error('Error:' + error.message);
      });
    });
}

function exportDNS(domain) {
  // Check if directory exists and then create it if it doesn't
  if (!fs.existsSync('./domains')) {
    fs.mkdirSync('./domains');
  }

  // Get domain records
  axios
    .get('zones/' + domain.id + '/dns_records/export', {
      baseURL: CLOUDFLARE_ENDPOINT,
      headers: {
        'X-Auth-Email': process.env.CLOUDFLARE_USER_EMAIL,
        'X-Auth-Key': process.env.CLOUDFLARE_API_KEY,
        'Content-Type': 'application/json',
      },
    })
    .then(function (response) {
      // Write Domain Files
      fs.writeFile(`./domains/${domain.name}.txt`, response.data, (err) => {
        if (err) console.log('Error writing file: ' + err);
      });
    })
    .catch(function (error) {
      // ERROR HANDLING
      error.response.data.errors.forEach((error) => {
        console.error('Error:' + error.message);
      });
    });
}

function addDomainsToList(domains) {
  domains.result.forEach((domain) => {
    // console.log('Domain: ' + domain.name + '   ID: ' + domain.id);
    domainList.push({ id: domain.id, name: domain.name });
  });

  return domainList;
}

function checkEnvironment() {
  // .env file exists, load into ENV
  if (fs.existsSync('.env')) {
    readEnvFile('.env')
  } else if(fs.existsSync(args[0])) {
    console.log('Using custom ENV file: ' + args[0]);
    readEnvFile(args[0])
  } else {
    console.log('No environment ( .env ) file found. Exiting');
    process.exit(1);
  }
}

function readEnvFile(envFilename) {
  require('dotenv').config({ path: envFilename });
  if (process.env.CLOUDFLARE_API_KEY && process.env.CLOUDFLARE_USER_EMAIL) {
    if (
      process.env.CLOUDFLARE_API_KEY == 'NULL' &&
      process.env.CLOUDFLARE_USER_EMAIL == 'NULL'
    ) {
      console.info('Please enter you own API and EMAIL in the .env file\n\n');
    }

    console.info('[Loaded environment data]\n\n');
  } else {
    console.log(
      'Required environment variables not set in .env file: CLOUDFLARE_API_KEY & CLOUDFLARE_USER_EMAIL'
    );
    process.exit(1);
  }
}
