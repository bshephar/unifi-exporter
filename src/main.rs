mod exporter;
mod unifi;

use ::clap::Parser;
use anyhow::anyhow;
use exporter::MetricsExporter;
use std::env;
use unifi::UnifiClient;

use actix_web::{App, HttpResponse, HttpServer, web};

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

async fn serve_metrics(
    exporter: web::Data<MetricsExporter>,
) -> Result<HttpResponse, actix_web::Error> {
    let body = exporter
        .render()
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    Ok(HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(body))
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let (endpoint, token) = load_config()?;

    let client = UnifiClient::new(&endpoint, token).await?;
    println!("Authenticating...");
    client.authenticate().await?;
    println!("✅ Authenticated!");

    println!("Iterating devices");
    let devices: unifi::DevicesResponse = fetch_devices(&client).await?;

    let exporter: MetricsExporter = MetricsExporter::new()?;

    for dev in &devices.data {
        println!("\nStats for device: {dev_name}", dev_name = dev.name);
        let raw_device_stats = client.get_device_stats(&dev.id.to_string()).await?;
        let device_stats: unifi::DeviceStats = serde_json::from_value(raw_device_stats)
            .map_err(|e| anyhow!("Failed to deserialize device stats response: {}", e))?;
        exporter.update_device_metrics(dev.name.as_str(), &device_stats);
    }

    let res = exporter.render();

    let exporter_data = web::Data::new(exporter);

    // Needs to be thread safe, so we can clone the data for each thread.
    HttpServer::new(move || {
        App::new()
            .app_data(exporter_data.clone())
            .route("/metrics", web::get().to(serve_metrics))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
    .map_err(anyhow::Error::from)
}
