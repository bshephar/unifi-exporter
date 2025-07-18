use ::clap::Parser;
use reqwest::Error;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use serde_json::Value;
use std::env;

#[derive(Parser, Debug)]
#[command(version, about)]
struct CmdArgs {
    #[arg(short, long)]
    device: String,

    #[arg(short, long)]
    backup: bool,

    #[arg(short, long)]
    endpoint: String,

    #[arg(short, long)]
    token: String,
}

// https://192.168.3.254/proxy/network/integration/v1/info
#[tokio::main]
async fn main() -> Result<(), Error> {
    let api_token = match env::var_os("API_TOKEN") {
        Some(v) => v.into_string().unwrap(),
        None => panic!("$API_TOKEN is not set"),
    };

    let mut headers = HeaderMap::new();
    headers.insert("X-API-KEY", HeaderValue::from_str(&api_token).unwrap());

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .default_headers(headers)
        .build()?;

    let response = client
        .get("https://192.168.3.254/proxy/network/integration/v1/info")
        .send()
        .await?;

    println!("Status: {}", response.status());
    let body = response.text().await?;

    println!("Body:{}", body);

    let sites_response = get_sites(&client, "192.168.3.254".to_string()).await;
    let mut site_id: Option<String> = None;

    match serde_json::from_str::<SitesResponse>(&sites_response.unwrap()) {
        Ok(sites_data) => {
            println!("Successfully retrieved and parsed sites data:");
            println!("{:#?}", sites_data);

            if let Some(first_site) = sites_data.data.first() {
                site_id = Some(first_site.id.clone());
                println!("Site ID: {}, Name: {}", first_site.id, first_site.name);
            }
        }
        Err(e) => {
            eprintln!("Error parsing JSON into SitesResponse struct: {}", e);
            println!("Raw body (could not parse as SitesResponse)",);
        }
    }

    let devices_response = get_devices(&client, "192.168.3.254".to_string(), site_id.unwrap())
        .await
        .unwrap()
        .to_string();
    println!("{}", devices_response);

    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub id: String,
    pub internal_reference: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SitesResponse {
    pub offset: u32,
    pub limit: u32,
    pub count: u32,
    pub total_count: u32,
    pub data: Vec<Site>,
}

// https://192.168.3.254/proxy/network/integration/v1/sites
async fn get_sites(client: &reqwest::Client, endpoint: String) -> Result<String, reqwest::Error> {
    let sites_response = client
        .get(format!(
            "https://{}/proxy/network/integration/v1/sites",
            endpoint
        ))
        .send()
        .await?;

    let sites_body = sites_response.text().await?;

    Ok(sites_body)
}

// https://192.168.3.254/proxy/network/integration/v1/sites/{siteId}/devices
async fn get_devices(
    client: &reqwest::Client,
    endpoint: String,
    site: String,
) -> Result<String, reqwest::Error> {
    let devices_response = client
        .get(format!(
            "https://{}/proxy/network/integration/v1/sites/{}/devices",
            endpoint, site
        ))
        .send()
        .await?;

    let devices_body = devices_response.text().await?;

    Ok(devices_body)
}
