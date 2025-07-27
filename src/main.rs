mod unifi;

use ::clap::Parser;
use anyhow::anyhow;
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
fn load_config() -> Result<(String, String), anyhow::Error> {
    let args = Args::parse();

    let endpoint = args
        .endpoint
        .or_else(|| env::var("UNIFI_API_ENDPOINT").ok())
        .unwrap_or_else(|| {
            let default = "https://192.168.3.254".to_string();
            println!("Using default endpoint: {}", default);
            default
        });

    let token = args
        .token
        .or_else(|| env::var("UNIFI_API_TOKEN").ok())
        .ok_or_else(|| {
            anyhow!("UNIFI_API_TOKEN not provided. Please pass --token or set UNIFI_API_TOKEN")
        })?;

    Ok((endpoint, token))
}

async fn fetch_devices(client: &UnifiClient) -> Result<unifi::DevicesResponse, anyhow::Error> {
    println!("🔍 Fetching devices...");
    let raw_devices = client.get_devices().await?;

    let devices: unifi::DevicesResponse = serde_json::from_value(raw_devices)
        .map_err(|e| anyhow!("Failed to deserialize devices response: {}", e))?;

    println!("\nDiscovered {} device(s):", devices.data.len());
    for device in &devices.data {
        println!("- {} ({}) [{}]", device.name, device.model, device.state);
    }

    Ok(devices)
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let (endpoint, token) = load_config()?;

    let mut client = UnifiClient::new(&endpoint, token).await?;
    println!("Authenticating...");
    client.authenticate().await?;
    println!("✅ Authenticated!");

    println!("Iterating devices");
    let devices: unifi::DevicesResponse = fetch_devices(&client).await?;

    for dev in &devices.data {
        println!("\nStats for device: {dev_name}", dev_name = dev.name);
        let raw_device_stats = client.get_device_stats(&dev.id.to_string()).await?;
        let device_stats: unifi::DeviceStats = serde_json::from_value(raw_device_stats)
            .map_err(|e| anyhow!("Failed to deserialize device stats response: {}", e))?;
        println!("{:#?}", device_stats);
    }

    println!("Done.");
    Ok(())
}
