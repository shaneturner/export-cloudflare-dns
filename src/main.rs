use clap::Parser;
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE},
    Client,
};
use serde::Deserialize;
use std::{
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process,
};

const CLOUDFLARE_ENDPOINT: &str = "https://api.cloudflare.com/client/v4/";

// Struct to deserialize the domain data from Cloudflare API
#[derive(Debug, Deserialize)]
struct Domain {
    id: String,
    name: String,
}

// Struct to deserialize the pagination information
#[derive(Debug, Deserialize)]
struct ResultInfo {
    page: u32,
    total_pages: u32,
    count: u32,
    total_count: u32,
}

// Struct to deserialize the main response
#[derive(Debug, Deserialize)]
struct CloudflareResponse {
    success: bool,
    // Cloudflare returns `result: null` and omits `result_info` on error responses
    result: Option<Vec<Domain>>,
    result_info: Option<ResultInfo>,
    errors: Vec<CloudflareError>,
}

// Struct to deserialize error messages
#[derive(Debug, Deserialize)]
struct CloudflareError {
    message: String,
}

/// Bulk export Cloudflare DNS records (BIND format) for every domain on an account.
#[derive(Parser)]
#[command(
    version,
    about,
    after_help = "Credentials are read from the environment or the env file:
  CLOUDFLARE_API_TOKEN     API token with Zone:Read and DNS:Read (preferred)
  CLOUDFLARE_API_KEY       Global API key (used if no token is set)
  CLOUDFLARE_USER_EMAIL    Account email (required with the API key)"
)]
struct Cli {
    /// Env file to load credentials from [default: .env, if present]
    env_file: Option<PathBuf>,

    /// Directory to write the <domain>.txt files to
    #[arg(short, long, default_value = "domains")]
    output: PathBuf,
}

enum Auth {
    Token(String),
    Key { key: String, email: String },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let client = create_client(&load_auth(cli.env_file.as_deref()))?;

    // Fetch data from Cloudflare
    println!("Getting List of domains from Cloudflare");
    println!("=======================================\n");

    // Get domain names from Cloudflare
    let domains = get_domains(&client).await?;

    // Export DNS records for each domain
    println!("Writing domain DNS files");
    fs::create_dir_all(&cli.output)?;

    for domain in domains {
        export_dns(&client, &domain, &cli.output).await?;
    }

    println!(
        "Domain DNS records complete. Please check the {} directory for your files",
        cli.output.display()
    );

    Ok(())
}

// Read a credential, treating empty values and the .env.example "NULL" placeholder as unset
fn credential(name: &str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.is_empty() && v != "NULL")
}

fn load_auth(custom_env: Option<&Path>) -> Auth {
    let env_file = custom_env.unwrap_or(Path::new(".env"));
    match dotenvy::from_path(env_file) {
        Ok(_) => println!("Using ENV file: {}", env_file.display()),
        // The default .env is optional when credentials are already in the environment
        Err(e) if e.not_found() && custom_env.is_none() => {}
        Err(e) if e.not_found() => {
            println!(
                "Error: Specified environment file '{}' not found",
                env_file.display()
            );
            println!("Please check the file path and try again");
            process::exit(1);
        }
        Err(e) => {
            println!("Error: Failed to load {}: {}", env_file.display(), e);
            println!("Please check that the file is formatted correctly");
            process::exit(1);
        }
    }

    let auth = if let Some(token) = credential("CLOUDFLARE_API_TOKEN") {
        Auth::Token(token)
    } else if let (Some(key), Some(email)) = (
        credential("CLOUDFLARE_API_KEY"),
        credential("CLOUDFLARE_USER_EMAIL"),
    ) {
        Auth::Key { key, email }
    } else {
        println!(
            "Error: No Cloudflare credentials found in {} or the environment",
            env_file.display()
        );
        println!("Create a .env file (you can copy .env.example) containing either:");
        println!("\nCLOUDFLARE_API_TOKEN=your_api_token_here\n");
        println!("or:");
        println!("\nCLOUDFLARE_API_KEY=your_api_key_here");
        println!("CLOUDFLARE_USER_EMAIL=your_email_here\n");
        println!("Run with --help for more options.");
        process::exit(1);
    };

    println!("[Loaded environment data]\n");
    auth
}

async fn get_domains(client: &Client) -> Result<Vec<Domain>, Box<dyn std::error::Error>> {
    let mut all_domains = Vec::new();
    let mut current_page = 1;

    loop {
        // Make request to Cloudflare API
        let response: reqwest::Response = match client
            .get(&format!("{}zones", CLOUDFLARE_ENDPOINT))
            .query(&[("page", current_page)])
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                println!("Error: Failed to connect to Cloudflare API: {}", e);
                println!("Please check your internet connection and try again");
                process::exit(1);
            }
        };

        let status = response.status();

        // Parse response
        let cf_response: CloudflareResponse = match response.json().await {
            Ok(resp) => resp,
            Err(e) => {
                println!("Error: Failed to parse Cloudflare API response: {}", e);
                println!("The API may have changed or returned unexpected data");
                process::exit(1);
            }
        };

        if !cf_response.success {
            println!("Error: Cloudflare API returned an unsuccessful response");
            for error in cf_response.errors {
                println!("  - {}", error.message);
            }
            if status == reqwest::StatusCode::UNAUTHORIZED
                || status == reqwest::StatusCode::FORBIDDEN
            {
                println!("Please check that your Cloudflare credentials are correct");
            }
            process::exit(1);
        }

        let (Some(domains), Some(page_info)) = (cf_response.result, cf_response.result_info) else {
            println!("Error: Cloudflare API response is missing domain or pagination data");
            process::exit(1);
        };
        println!("Fetching batch of {} DNS records ...", page_info.count);

        // Add domains to our list
        all_domains.extend(domains);

        // Check if there are more pages
        if page_info.page >= page_info.total_pages {
            println!("Fetched {} domains.", page_info.total_count);
            break;
        }

        current_page = page_info.page + 1;
    }

    Ok(all_domains)
}

async fn export_dns(
    client: &Client,
    domain: &Domain,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get DNS records for domain
    let response = match client
        .get(&format!(
            "{}zones/{}/dns_records/export",
            CLOUDFLARE_ENDPOINT, domain.id
        ))
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            println!(
                "Error: Failed to fetch DNS records for domain {}: {}",
                domain.name, e
            );
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to fetch DNS records for domain {}", domain.name),
            )));
        }
    };

    // Check if the response was successful
    if !response.status().is_success() {
        println!(
            "Error: Cloudflare API returned status code {} when fetching DNS records for {}",
            response.status(),
            domain.name
        );
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Failed to fetch DNS records for domain {} - status: {}",
                domain.name,
                response.status()
            ),
        )));
    }

    // Get the response as text
    let dns_data = match response.text().await {
        Ok(text) => text,
        Err(e) => {
            println!(
                "Error: Failed to read DNS records for domain {}: {}",
                domain.name, e
            );
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to read DNS records for domain {}", domain.name),
            )));
        }
    };

    // Write to file
    let file_path = output.join(format!("{}.txt", domain.name));
    let mut file = match File::create(&file_path) {
        Ok(f) => f,
        Err(e) => {
            println!(
                "Error: Failed to create file {}: {}",
                file_path.display(),
                e
            );
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create file {}", file_path.display()),
            )));
        }
    };

    match file.write_all(dns_data.as_bytes()) {
        Ok(_) => println!("Successfully exported DNS records for {}", domain.name),
        Err(e) => {
            println!(
                "Error: Failed to write DNS records for domain {} to file: {}",
                domain.name, e
            );
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write DNS records for domain {}", domain.name),
            )));
        }
    };

    Ok(())
}

fn create_client(auth: &Auth) -> Result<Client, Box<dyn std::error::Error>> {
    let mut headers = HeaderMap::new();

    let valid = match auth {
        Auth::Token(token) => HeaderValue::from_str(&format!("Bearer {}", token))
            .map(|v| headers.insert(AUTHORIZATION, v))
            .is_ok(),
        Auth::Key { key, email } => {
            match (HeaderValue::from_str(key), HeaderValue::from_str(email)) {
                (Ok(key), Ok(email)) => {
                    headers.insert("X-Auth-Key", key);
                    headers.insert("X-Auth-Email", email);
                    true
                }
                _ => false,
            }
        }
    };

    if !valid {
        println!("Error: Cloudflare credentials contain invalid characters");
        println!("Please check the values in your .env file");
        process::exit(1);
    }

    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    // Create and return client
    Ok(Client::builder().default_headers(headers).build()?)
}
