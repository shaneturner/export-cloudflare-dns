use clap::Parser;
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE},
    Client,
};
use serde::Deserialize;
use std::{
    env,
    fs::{self, File},
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    process,
};

const CLOUDFLARE_ENDPOINT: &str = "https://api.cloudflare.com/client/v4/";
const CONFIG_FILE: &str = "cloudflare-export.env";
const TOKEN_URL: &str = "https://dash.cloudflare.com/profile/api-tokens";

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
    after_help = "Credentials are loaded from the first of:
  1. ENV_FILE, if given
  2. cloudflare-export.env next to this program (created by --setup)
  3. .env in the current directory
  4. Environment variables

Credential variables:
  CLOUDFLARE_API_TOKEN     API token with Zone:Read and DNS:Read (preferred)
  CLOUDFLARE_API_KEY       Global API key (used if no token is set)
  CLOUDFLARE_USER_EMAIL    Account email (required with the API key)"
)]
struct Cli {
    /// Env file to load credentials from
    env_file: Option<PathBuf>,

    /// Directory to write the <domain>.txt files to
    #[arg(short, long, default_value = "domains")]
    output: PathBuf,

    /// Guide you through creating an API token and saving it
    #[arg(long)]
    setup: bool,
}

enum Auth {
    Token(String),
    Key { key: String, email: String },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let interactive = io::stdin().is_terminal();

    let auth = if cli.setup {
        if !interactive {
            println!("Error: --setup needs an interactive terminal");
            exit(1);
        }
        let auth = setup().await;
        if !confirm("\nExport DNS records now?") {
            exit(0);
        }
        auth
    } else if let Some(auth) = load_auth(cli.env_file.as_deref()) {
        auth
    } else {
        println!("No Cloudflare credentials found.\n");
        println!("Run this program with --setup to create {}.", CONFIG_FILE);
        println!("Run with --help for other ways to provide credentials.\n");
        if !(interactive && confirm("Run setup now?")) {
            exit(1);
        }
        setup().await
    };

    if let Err(e) = run(&auth, &cli.output).await {
        println!("Error: {}", e);
        exit(1);
    }
    pause_if_own_console();
}

async fn run(auth: &Auth, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let client = create_client(auth)?;

    // Fetch data from Cloudflare
    println!("Getting List of domains from Cloudflare");
    println!("=======================================\n");

    // Get domain names from Cloudflare
    let domains = get_domains(&client).await?;

    // Export DNS records for each domain
    println!("Writing domain DNS files");
    fs::create_dir_all(output)?;

    for domain in domains {
        export_dns(&client, &domain, output).await?;
    }

    println!(
        "Domain DNS records complete. Your files are in {}",
        fs::canonicalize(output)?.display()
    );

    Ok(())
}

// Exit the process, first pausing so a double-clicked console window stays readable
fn exit(code: i32) -> ! {
    pause_if_own_console();
    process::exit(code)
}

// On Windows, a console owned only by this process means it was launched by double-click
// and the window would close as soon as we exit
#[cfg(windows)]
fn pause_if_own_console() {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetConsoleProcessList(list: *mut u32, count: u32) -> u32;
    }
    let mut pids = [0u32; 2];
    if unsafe { GetConsoleProcessList(pids.as_mut_ptr(), 2) } == 1 {
        print!("\nPress Enter to exit...");
        let _ = io::stdout().flush();
        let _ = io::stdin().read_line(&mut String::new());
    }
}

#[cfg(not(windows))]
fn pause_if_own_console() {}

// Ask a yes/no question, defaulting to yes
fn confirm(question: &str) -> bool {
    print!("{} [Y/n] ", question);
    let _ = io::stdout().flush();
    let mut answer = String::new();
    io::stdin().read_line(&mut answer).is_ok()
        && matches!(answer.trim().to_lowercase().as_str(), "" | "y" | "yes")
}

// The config file created by --setup lives next to the executable so it is found
// regardless of the directory the program is launched from
fn config_path() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(CONFIG_FILE)))
        .unwrap_or_else(|| PathBuf::from(CONFIG_FILE))
}

// Read a credential, treating empty values and the .env.example "NULL" placeholder as unset
fn credential(name: &str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.is_empty() && v != "NULL")
}

fn load_auth(custom_env: Option<&Path>) -> Option<Auth> {
    let env_file = match custom_env {
        Some(path) if !path.exists() => {
            println!(
                "Error: Specified environment file '{}' not found",
                path.display()
            );
            println!("Please check the file path and try again");
            exit(1);
        }
        Some(path) => Some(path.to_path_buf()),
        None => [config_path(), PathBuf::from(".env")]
            .into_iter()
            .find(|p| p.exists()),
    };

    if let Some(env_file) = &env_file {
        if let Err(e) = dotenvy::from_path(env_file) {
            println!("Error: Failed to load {}: {}", env_file.display(), e);
            println!("Please check that the file is formatted correctly");
            exit(1);
        }
        println!("Using ENV file: {}", env_file.display());
    }

    let auth = if let Some(token) = credential("CLOUDFLARE_API_TOKEN") {
        Auth::Token(token)
    } else if let (Some(key), Some(email)) = (
        credential("CLOUDFLARE_API_KEY"),
        credential("CLOUDFLARE_USER_EMAIL"),
    ) {
        Auth::Key { key, email }
    } else {
        return None;
    };

    println!("[Loaded environment data]\n");
    Some(auth)
}

// Interactive wizard: ask for an API token, verify it, and save it to the config file
async fn setup() -> Auth {
    println!("\nCloudflare DNS Exporter setup");
    println!("=============================\n");
    println!("You need a Cloudflare API token that can read your zones and DNS records:");
    println!("  1. Open {}", TOKEN_URL);
    println!("  2. Select \"Create Token\" and then \"Create Custom Token\"");
    println!("  3. Add the permissions  Zone > Zone > Read  and  Zone > DNS > Read");
    println!("  4. Under Zone Resources choose \"All zones\" and create the token\n");

    let token = loop {
        let token = match rpassword::prompt_password("Paste your API token (input is hidden): ") {
            Ok(t) => t.trim().to_string(),
            Err(e) => {
                println!("Error: Failed to read the token: {}", e);
                exit(1);
            }
        };
        if token.is_empty() {
            println!("No token entered. Setup cancelled.");
            exit(1);
        }

        print!("Checking token with Cloudflare... ");
        let _ = io::stdout().flush();
        let auth = Auth::Token(token.clone());
        let client = match create_client(&auth) {
            Ok(c) => c,
            Err(e) => {
                println!("\nError: {}", e);
                exit(1);
            }
        };
        match fetch_zones_page(&client, 1).await {
            Ok(_) => {
                println!("OK\n");
                break token;
            }
            Err(e) => println!(
                "failed\n{}\nPlease try again, or press Enter to cancel.\n",
                e
            ),
        }
    };

    let path = config_path();
    if let Err(e) = save_config(&path, &token) {
        println!("Error: Could not save {}: {}", path.display(), e);
        println!(
            "Move this program to a folder you can write to, or set CLOUDFLARE_API_TOKEN yourself."
        );
        exit(1);
    }
    println!("Saved your token to {}", path.display());
    println!("Keep this file private: anyone with it can read your DNS records.");

    Auth::Token(token)
}

fn save_config(path: &Path, token: &str) -> io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    // Keep the token readable only by the current user
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);

    let mut file = options.open(path)?;
    writeln!(
        file,
        "# Cloudflare DNS Exporter credentials (created by --setup)"
    )?;
    writeln!(file, "# API token with Zone:Read and DNS:Read permissions")?;
    writeln!(file, "CLOUDFLARE_API_TOKEN={}", token)
}

// Fetch one page of zones, returning a user-facing message on failure
async fn fetch_zones_page(client: &Client, page: u32) -> Result<(Vec<Domain>, ResultInfo), String> {
    let response = client
        .get(format!("{}zones", CLOUDFLARE_ENDPOINT))
        .query(&[("page", page)])
        .send()
        .await
        .map_err(|e| {
            format!(
                "Error: Failed to connect to Cloudflare API: {}\nPlease check your internet connection and try again",
                e
            )
        })?;

    let status = response.status();

    let cf_response: CloudflareResponse = response.json().await.map_err(|e| {
        format!(
            "Error: Failed to parse Cloudflare API response: {}\nThe API may have changed or returned unexpected data",
            e
        )
    })?;

    if !cf_response.success {
        let mut message = String::from("Error: Cloudflare API returned an unsuccessful response");
        for error in cf_response.errors {
            message.push_str(&format!("\n  - {}", error.message));
        }
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            message.push_str("\nPlease check that your Cloudflare credentials are correct");
        }
        return Err(message);
    }

    match (cf_response.result, cf_response.result_info) {
        (Some(domains), Some(page_info)) => Ok((domains, page_info)),
        _ => Err("Error: Cloudflare API response is missing domain or pagination data".into()),
    }
}

async fn get_domains(client: &Client) -> Result<Vec<Domain>, Box<dyn std::error::Error>> {
    let mut all_domains = Vec::new();
    let mut current_page = 1;

    loop {
        let (domains, page_info) = match fetch_zones_page(client, current_page).await {
            Ok(page) => page,
            Err(message) => {
                println!("{}", message);
                exit(1);
            }
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
        exit(1);
    }

    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    // Create and return client
    Ok(Client::builder().default_headers(headers).build()?)
}
