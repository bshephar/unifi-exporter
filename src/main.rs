mod unifi;

use ::clap::Parser;
use reqwest::Error;
use std::env;
use unifi::{SitesResponse, UnifiClient};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Unifi controller endpoint URL (e.g., "https://192.168.3.254")
    #[arg(short, long)]
    endpoint: Option<String>,

    /// Unifi API Token
    #[arg(short, long)]
    token: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let unifi_endpoint = args.endpoint.unwrap_or_else(|| {
        env::var("UNIFI_API_ENDPOINT").unwrap_or_else(|_| {
            let default_endpoint = "https://192.168.3.254".to_string();
            println!(
                "No endpoint provided via CLI or UNIFI_API_ENDPOINT env var. Using default: {}",
                default_endpoint
            );
            default_endpoint
        })
    });

    let unifi_api_token = args.token.or_else(|| env::var("UNIFI_API_TOKEN").ok())
        .ok_or_else(|| anyhow::anyhow!("UNIFI_API_TOKEN not provided. Please set it via --api-token CLI option or UNIFI_API_TOKEN environment variable."))?;

    let mut client = UnifiClient::new(&unifi_endpoint, unifi_api_token)?;
    println!("Attempting to authenticate...");
    client.authenticate().await?;

    println!("Successfully authenticated with Unifi controller!");
    println!("\nFetching Unifi sites...");
    match client.get_sites().await {
        Ok(sites) => {
            println!("Successfully fetched sites:");

            // Pretty print the JSON response
            println!("{}", serde_json::to_string_pretty(&sites)?);

            // Try to extract a site ID to fetch devices
            if let Some(sites_array) = sites["data"].as_array() {
                if let Some(first_site) = sites_array.first() {
                    if let Some(site_id) = first_site["id"].as_str() {
                        println!("\nUsing first site ID: {}", site_id);

                        // 4. Fetch devices for the first site
                        println!("Fetching devices for site '{}'...", site_id);
                        match client.get_devices(site_id).await {
                            Ok(devices) => {
                                println!("Successfully fetched devices:");
                                println!("{}", serde_json::to_string_pretty(&devices)?);
                            }
                            Err(e) => eprintln!("Error fetching devices: {:?}", e),
                        }
                    } else {
                        println!("Could not find 'id' for the first site.");
                    }
                } else {
                    println!("No sites found in the response.");
                }
            } else {
                println!("'data' field not found or not an array in sites response.");
            }
        }
        Err(e) => eprintln!("Error fetching sites: {:?}", e),
    }

    println!("\nUnifi Exporter Application finished.");

    Ok(())
}
