use anyhow::{bail, Context, Result};
use tracing::{debug, warn};

use crate::scanner::client::ScannerClient;
use crate::scanner::models::destination::{
    build_destination_xml, parse_destinations_list, parse_max_destinations, RegisteredDestination,
};

const CAPS_PATH: &str = "/WalkupScanToComp/WalkupScanToCompCaps";
const DESTINATIONS_PATH: &str = "/WalkupScanToComp/WalkupScanToCompDestinations";

/// Fetch the maximum number of destinations the scanner supports.
pub async fn get_max_destinations(client: &ScannerClient) -> Result<u32> {
    let resp = client.get(CAPS_PATH).await?;
    let status = resp.status();
    if !status.is_success() {
        bail!("GET {CAPS_PATH} returned {status}");
    }
    let xml = resp.text().await?;
    Ok(parse_max_destinations(&xml).unwrap_or(5))
}

/// List all currently registered destinations.
pub async fn list_destinations(client: &ScannerClient) -> Result<Vec<RegisteredDestination>> {
    let resp = client.get(DESTINATIONS_PATH).await?;
    let status = resp.status();
    if !status.is_success() {
        bail!("GET {DESTINATIONS_PATH} returned {status}");
    }
    let xml = resp.text().await?;
    debug!("list_destinations response:\n{xml}");
    parse_destinations_list(&xml)
}

/// Register a new destination with the given label.
/// Returns the resource URI path (e.g. `/WalkupScanToComp/WalkupScanToCompDestinations/1`).
pub async fn register_destination(client: &ScannerClient, label: &str) -> Result<String> {
    let xml = build_destination_xml(label);
    debug!("register_destination POST body:\n{xml}");
    let url = format!("{}{}", client.base_url(), DESTINATIONS_PATH);
    let resp = client.post_xml_url(&url, xml).await?;
    let status = resp.status();
    if status.as_u16() != 201 {
        let body = resp.text().await.unwrap_or_default();
        bail!(
            "POST {} returned {} (expected 201)\nResponse body: {body}",
            DESTINATIONS_PATH,
            status
        );
    }

    // The scanner returns the resource as an absolute URL in Location.
    // We store just the path portion.
    let location = resp
        .headers()
        .get("Location")
        .context("register_destination: no Location header in 201 response")?
        .to_str()
        .context("Location header is not valid UTF-8")?
        .to_string();

    // Strip the host if it's an absolute URL.
    let path = if location.starts_with("http") {
        let parsed = reqwest::Url::parse(&location)?;
        parsed.path().to_string()
    } else {
        location
    };

    debug!("registered destination {:?} → {path}", label);
    Ok(path)
}

/// Delete a destination by its resource URI path.
pub async fn delete_destination(client: &ScannerClient, resource_uri: &str) -> Result<()> {
    let status = client.delete(resource_uri).await?;
    if !status.is_success() && status.as_u16() != 404 {
        warn!("DELETE {resource_uri} returned {status}");
    }
    Ok(())
}

/// The type of WalkupScanToCompEvent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompEventType {
    HostSelected,
    ScanRequested,
    ScanNewPageRequested,
    ScanPagesComplete,
    Unknown,
}

/// GET the WalkupScanToCompEvent endpoint and parse the event type.
pub async fn get_comp_event_type(
    client: &ScannerClient,
    comp_event_uri: &str,
) -> Result<CompEventType> {
    let resp = client.get(comp_event_uri).await?;
    let status = resp.status();
    if !status.is_success() {
        bail!("GET {comp_event_uri} returned {status}");
    }
    let xml = resp.text().await?;
    debug!("WalkupScanToCompEvent response:\n{xml}");
    Ok(parse_comp_event_type(&xml))
}

fn parse_comp_event_type(xml: &str) -> CompEventType {
    let doc = match roxmltree::Document::parse(xml) {
        Ok(d) => d,
        Err(_) => return CompEventType::Unknown,
    };
    for node in doc.root_element().descendants() {
        if node.tag_name().name() == "WalkupScanToCompEventType" {
            return match node.text().unwrap_or("") {
                "HostSelected" => CompEventType::HostSelected,
                "ScanRequested" => CompEventType::ScanRequested,
                "ScanNewPageRequested" => CompEventType::ScanNewPageRequested,
                "ScanPagesComplete" => CompEventType::ScanPagesComplete,
                _ => CompEventType::Unknown,
            };
        }
    }
    CompEventType::Unknown
}
