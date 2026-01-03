mod exporter;
mod unifi;

use ::clap::Parser;
use anyhow::anyhow;
use exporter::MetricsExporter;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep};
use unifi::UnifiClient;

use actix_web::{App, HttpResponse, HttpServer, web};
use tracing::{error, info, warn};
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
const SLEEP_TIME: u64 = 300;

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
    client: &Arc<UnifiClient>,
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

    let client = Arc::new(UnifiClient::new(&endpoint, token).await?);
    info!("Authenticating...");
    client.authenticate().await?;
    info!("Authenticated!");

    let shared_data = Arc::new(RwLock::new(String::new()));

    let loop_client = client.clone();
    let data_ptr = shared_data.clone();

    tokio::spawn(async move {
        loop {
            match fetch_devices(&loop_client).await {
                Ok(devices) => {
                    match render_exporter_data(devices, &loop_client).await {
                        Ok(new_exporter_obj) => {
                            // Convert the exporter object into the actual String
                            match new_exporter_obj.render() {
                                Ok(rendered_string) => {
                                    let mut w = data_ptr.write().await;
                                    *w = rendered_string;
                                    info!("Metrics cache updated.");
                                }
                                Err(e) => error!("Failed to format metrics: {}", e),
                            }
                        }
                        Err(e) => error!("Rendering error: {}", e),
                    }
                }
                Err(e) if e.to_string().contains("401") => {
                    warn!("Session expired, re-authenticating...");
                    let _ = loop_client.authenticate().await;
                }
                Err(e) => error!("Fetch error: {}", e),
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(SLEEP_TIME)).await;
        }
    });

    info!("Exposing unifi metrics on: {}", EXPOSE_ADDRESS);

    HttpServer::new(move || {
        App::new()
            // Wrap the Arc in web::Data so Actix can extract it in the handler
            .app_data(web::Data::new(shared_data.clone()))
            .route("/metrics", web::get().to(serve_metrics))
    })
    .bind(EXPOSE_ADDRESS)?
    .run()
    .await
    .map_err(anyhow::Error::from)
}
