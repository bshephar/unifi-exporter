mod exporter;
mod unifi;

use ::clap::Parser;
use anyhow::anyhow;
use exporter::MetricsExporter;
use std::env;
use unifi::UnifiClient;

use actix_web::{App, HttpResponse, HttpServer, web};
use tracing::info;
use tracing_subscriber;

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

const EXPOSE_ADDRESS: &str = "0.0.0.0:8080";

fn load_config() -> Result<(String, String), anyhow::Error> {
    let args = Args::parse();

    let endpoint = args
        .endpoint
        .or_else(|| env::var("UNIFI_API_ENDPOINT").ok())
        .unwrap_or_else(|| {
            let default = "https://192.168.3.254".to_string();
            info!("Using default endpoint: {}", default);
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
    info!("Fetching devices...");
    let raw_devices = client.get_devices().await?;

    let devices: unifi::DevicesResponse = serde_json::from_value(raw_devices)
        .map_err(|e| anyhow!("Failed to deserialize devices response: {}", e))?;

    info!("Discovered {} device(s):", devices.data.len());
    for device in &devices.data {
        info!("- {} ({}) [{}]", device.name, device.model, device.state);
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

async fn render_exporter_data(
    devices: unifi::DevicesResponse,
    client: UnifiClient,
) -> Result<actix_web::web::Data<MetricsExporter>, anyhow::Error> {
    let exporter: MetricsExporter = MetricsExporter::new()?;

    for dev in &devices.data {
        let raw_device_stats = client.get_device_stats(&dev.id.to_string()).await?;
        let device_stats: unifi::DeviceStats = serde_json::from_value(raw_device_stats)
            .map_err(|e| anyhow!("Failed to deserialize device stats response: {}", e))?;
        exporter.update_device_metrics(dev.name.as_str(), &device_stats);
    }

    exporter.render()?;

    let exporter_data = web::Data::new(exporter);
    Ok(exporter_data)
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let (endpoint, token) = load_config()?;
    tracing_subscriber::fmt::init();

    let client = UnifiClient::new(&endpoint, token).await?;
    info!("Authenticating...");
    client.authenticate().await?;
    info!("Authenticated!");

    let devices: unifi::DevicesResponse = fetch_devices(&client).await?;

    let exporter_data = render_exporter_data(devices, client).await?;

    info!("Exposing unifi metrics on: {}", EXPOSE_ADDRESS);

    // Needs to be thread safe, so we can clone the data for each thread.
    HttpServer::new(move || {
        App::new()
            .app_data(exporter_data.clone())
            .route("/metrics", web::get().to(serve_metrics))
    })
    .bind(EXPOSE_ADDRESS)?
    .run()
    .await
    .map_err(anyhow::Error::from)
}
