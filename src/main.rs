use dotenv::dotenv;
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
    Client,
};
use serde::Deserialize;
use std::{
    env,
    fs::{self, File},
    io::Write,
    path::Path,
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
    result: Vec<Domain>,
    result_info: ResultInfo,
    errors: Vec<CloudflareError>,
}

// Struct to deserialize error messages
#[derive(Debug, Deserialize)]
struct CloudflareError {
    message: String,
}

// No need for DnsExportResponse struct as we're handling the DNS export data as plain text

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check environment
    check_environment();

    // Fetch data from Cloudflare
    println!("Getting List of domains from Cloudflare");
    println!("=======================================\n");

    // Get domain names from Cloudflare
    let domains = get_domains().await?;

    // Export DNS records for each domain
    println!("Writing domain DNS files");

    // Create domains directory if it doesn't exist
    if !Path::new("./domains").exists() {
        fs::create_dir("./domains")?;
    }

    for domain in domains {
        export_dns(&domain).await?;
    }

    println!("Domain DNS records complete. Please check the /domains directory for your files");

    Ok(())
}

fn check_environment() {
    // Check for custom env file from command line args
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let env_path = &args[1];
        if Path::new(env_path).exists() {
            println!("Using custom ENV file: {}", env_path);
            if let Err(_) = dotenv::from_path(env_path) {
                println!(
                    "Error: Failed to load environment variables from {}",
                    env_path
                );
                println!("Please check that the file exists and is formatted correctly");
                process::exit(1);
            }
        } else {
            println!("Error: Specified environment file '{}' not found", env_path);
            println!("Please check the file path and try again");
            process::exit(1);
        }
    } else {
        // Use default .env file
        if let Err(_) = dotenv() {
            println!("No environment (.env) file found.");
            println!("Please create a .env file with your Cloudflare API credentials.");
            println!("You can copy the .env.example file as a starting point:");
            println!("\ncp .env.example .env\n");
            println!("Then edit the .env file to add your credentials.");
            process::exit(1);
        }
    }

    // Check if required environment variables are set
    let api_key = match env::var("CLOUDFLARE_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("Error: CLOUDFLARE_API_KEY not found in environment");
            println!("Please make sure your .env file contains:");
            println!("CLOUDFLARE_API_KEY=your_api_key_here");
            process::exit(1);
        }
    };

    let email = match env::var("CLOUDFLARE_USER_EMAIL") {
        Ok(email) => email,
        Err(_) => {
            println!("Error: CLOUDFLARE_USER_EMAIL not found in environment");
            println!("Please make sure your .env file contains:");
            println!("CLOUDFLARE_USER_EMAIL=your_email_here");
            process::exit(1);
        }
    };

    // Give a warning if default values from .env.example are still being used
    if api_key == "NULL" || email == "NULL" {
        println!("Warning: You appear to be using default placeholder values in your .env file");
        println!("Please update your .env file with your actual Cloudflare credentials:");
        println!("CLOUDFLARE_API_KEY=your_api_key_here");
        println!("CLOUDFLARE_USER_EMAIL=your_email_here");
        println!("\nExiting. Please update your credentials and try again.");
        process::exit(1);
    }

    println!("[Loaded environment data]\n");
}

async fn get_domains() -> Result<Vec<Domain>, Box<dyn std::error::Error>> {
    let mut all_domains = Vec::new();
    let mut current_page = 1;

    // Create HTTP client
    let client = create_client()?;

    loop {
        // Make request to Cloudflare API
        let response = match client
            .get(&format!("{}zones", CLOUDFLARE_ENDPOINT))
            .query(&[("page", current_page)])
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                // Check if this is an authentication error
                if e.is_status() {
                    if let Some(status) = e.status() {
                        if status == reqwest::StatusCode::UNAUTHORIZED
                            || status == reqwest::StatusCode::FORBIDDEN
                        {
                            println!("Error: Authentication failed with Cloudflare API");
                            println!("Please check that your API key and email are correct");
                            process::exit(1);
                        }
                    }
                }

                println!("Error: Failed to connect to Cloudflare API: {}", e);
                println!("Please check your internet connection and try again");
                process::exit(1);
            }
        };

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
            process::exit(1);
        }

        let page_info = &cf_response.result_info;
        println!("Fetching batch of {} DNS records ...", page_info.count);

        // Add domains to our list
        all_domains.extend(cf_response.result);

        // Check if there are more pages
        if page_info.page >= page_info.total_pages {
            println!("Fetched {} domains.", page_info.total_count);
            break;
        }

        current_page = page_info.page + 1;
    }

    Ok(all_domains)
}

async fn export_dns(domain: &Domain) -> Result<(), Box<dyn std::error::Error>> {
    // Create HTTP client
    let client = create_client()?;

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
    let file_path = format!("./domains/{}.txt", domain.name);
    let mut file = match File::create(&file_path) {
        Ok(f) => f,
        Err(e) => {
            println!("Error: Failed to create file {}: {}", file_path, e);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create file {}", file_path),
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

fn create_client() -> Result<Client, Box<dyn std::error::Error>> {
    // Get API key and email from environment variables
    let api_key = match env::var("CLOUDFLARE_API_KEY") {
        Ok(key) => {
            if key.is_empty() || key == "NULL" {
                println!("Error: Cloudflare API key is empty or set to NULL");
                println!("Please update your .env file with a valid API key");
                process::exit(1);
            }
            key
        }
        Err(_) => {
            println!("Error: CLOUDFLARE_API_KEY environment variable not found");
            println!("Please make sure your .env file contains CLOUDFLARE_API_KEY");
            process::exit(1);
        }
    };

    let email = match env::var("CLOUDFLARE_USER_EMAIL") {
        Ok(email) => {
            if email.is_empty() || email == "NULL" {
                println!("Error: Cloudflare user email is empty or set to NULL");
                println!("Please update your .env file with a valid email address");
                process::exit(1);
            }
            email
        }
        Err(_) => {
            println!("Error: CLOUDFLARE_USER_EMAIL environment variable not found");
            println!("Please make sure your .env file contains CLOUDFLARE_USER_EMAIL");
            process::exit(1);
        }
    };

    // Create headers
    let mut headers = HeaderMap::new();

    // Handle potential header creation errors with user-friendly messages
    match HeaderValue::from_str(&email) {
        Ok(header_value) => headers.insert("X-Auth-Email", header_value),
        Err(_) => {
            println!("Error: Invalid email format for Cloudflare header");
            println!("Please check your email address in the .env file");
            process::exit(1);
        }
    };

    match HeaderValue::from_str(&api_key) {
        Ok(header_value) => headers.insert("X-Auth-Key", header_value),
        Err(_) => {
            println!("Error: Invalid API key format for Cloudflare header");
            println!("Please check your API key in the .env file");
            process::exit(1);
        }
    };

    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    // Create and return client
    Ok(Client::builder().default_headers(headers).build()?)
}
