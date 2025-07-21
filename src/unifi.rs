use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use url::Url;

const API_PATH_INFO: &str = "/proxy/network/integration/v1/info";
const API_PATH_SITES: &str = "/proxy/network/integration/v1/sites";
const API_PATH_DEVICES: &str = "/proxy/network/integration/v1/sites/{}/devices";

pub struct UnifiClient {
    client: Client,
    endpoint: Url,
    api_token: String,
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

impl UnifiClient {
    /// Creates a new `UnifiClient` instance.
    ///
    /// # Arguments
    /// * `endpoint_str` - The base URL of your Unifi controller (e.g., "https://192.168.3.254").
    /// * `api_token` - The API token for Unifi controller authentication.
    ///
    /// # Returns
    /// A `Result` containing the `UnifiClient` instance or an `anyhow::Error` if the endpoint URL is invalid.
    pub fn new(endpoint_str: &str, api_token: String) -> Result<Self> {
        let endpoint = Url::parse(endpoint_str)?;

        // Initialize a reqwest client.
        // For API token authentication, we don't need a cookie jar for session management,
        // but we still need to handle self-signed certificates.
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;

        Ok(Self {
            client,
            endpoint,
            api_token,
        })
    }

    pub async fn authenticate(&self) -> Result<()> {
        let test_url = self
            .endpoint
            .join(API_PATH_INFO)
            .map_err(|e| anyhow!("Failed to construct test URL: {}", e))?;

        println!(
            "Attempting to authenticate with Unifi controller at: {}",
            test_url
        );

        let response = self
            .client
            .get(test_url)
            .header("X-API-KEY", &self.api_token)
            .send()
            .await?;

        if response.status().is_success() {
            println!("Authentication successful with API token!");
            Ok(())
        } else {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read response body".to_string());
            Err(anyhow!(
                "API Token authentication failed! Status: {}, Body: {}",
                status,
                body
            ))
        }
    }
    pub async fn get_sites(&self) -> Result<Value> {
        let sites_url = self
            .endpoint
            .join(API_PATH_SITES)
            .map_err(|e| anyhow!("Failed to construct sites URL: {}", e))?;

        println!("Fetching sites from: {}", sites_url);

        let response = self
            .client
            .get(sites_url)
            .header("X-API-KEY", &self.api_token)
            .send()
            .await?;

        if response.status().is_success() {
            let sites_body: Value = response.json().await?;
            Ok(sites_body)
        } else {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read response body".to_string());
            Err(anyhow!(
                "Failed to fetch sites! Status: {}, Body: {}",
                status,
                body
            ))
        }
    }

    /// Fetches the list of devices for a specific site from the Unifi controller.
    ///
    /// # Arguments
    /// * `site_id` - The ID of the Unifi site (e.g., "default").
    ///
    /// # Returns
    /// A `Result` containing a `serde_json::Value` representing the devices data,
    /// or an `anyhow::Error` on failure.
    pub async fn get_devices(&self, site_id: &str) -> Result<Value> {
        // I don't _love_ this, I feel like I'm fighting against the language here. But for now...
        let relative_path = API_PATH_DEVICES.replace("{}", site_id);
        let devices_url = self
            .endpoint
            .join(&relative_path)
            .map_err(|e| anyhow!("Failed to construct devices URL: {}", e))?;

        println!(
            "Fetching devices for site '{}' from: {}",
            site_id, devices_url
        );

        let response = self
            .client
            .get(devices_url)
            .header("X-API-KEY", &self.api_token)
            .send()
            .await?;

        if response.status().is_success() {
            let devices_body: Value = response.json().await?;
            Ok(devices_body)
        } else {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read response body".to_string());
            Err(anyhow!(
                "Failed to fetch devices for site '{}'! Status: {}, Body: {}",
                site_id,
                status,
                body
            ))
        }
    }
}
