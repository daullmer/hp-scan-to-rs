use anyhow::{bail, Context, Result};
use tracing::debug;

use crate::scanner::client::ScannerClient;
use crate::scanner::models::capabilities::{parse_capabilities, ScannerCapabilities};
use crate::scanner::models::scanner_status::{
    parse_scan_image_info, parse_scanner_status, ScanImageInfo, ScannerStatus,
};

/// GET `/eSCL/ScannerCapabilities`
pub async fn get_capabilities(client: &ScannerClient) -> Result<ScannerCapabilities> {
    let resp = client.get("/eSCL/ScannerCapabilities").await?;
    let status = resp.status();
    if !status.is_success() {
        bail!("GET /eSCL/ScannerCapabilities returned {status}");
    }
    let xml = resp.text().await?;
    debug!("ScannerCapabilities:\n{xml}");
    parse_capabilities(&xml)
}

/// GET `/eSCL/ScannerStatus`
pub async fn get_scanner_status(client: &ScannerClient) -> Result<ScannerStatus> {
    let resp = client.get("/eSCL/ScannerStatus").await?;
    let status = resp.status();
    if !status.is_success() {
        bail!("GET /eSCL/ScannerStatus returned {status}");
    }
    let xml = resp.text().await?;
    parse_scanner_status(&xml)
}

/// POST `/eSCL/ScanJobs` with the given XML body.
/// Returns the job URI path (stripped of host).
pub async fn create_scan_job(client: &ScannerClient, settings_xml: String) -> Result<String> {
    let url = format!("{}/eSCL/ScanJobs", client.base_url());
    let resp = client.post_xml_url(&url, settings_xml).await?;
    let status = resp.status();
    if status.as_u16() != 201 {
        bail!("POST /eSCL/ScanJobs returned {} (expected 201)", status);
    }

    let location = resp
        .headers()
        .get("Location")
        .context("create_scan_job: no Location header in 201 response")?
        .to_str()
        .context("Location is not valid UTF-8")?
        .to_string();

    // Strip host from absolute URL.
    let path = if location.starts_with("http") {
        let parsed = reqwest::Url::parse(&location)?;
        parsed.path().to_string()
    } else {
        location
    };

    debug!("scan job created: {path}");
    Ok(path)
}

/// GET `{job_path}/NextDocument`
/// Returns the raw JPEG bytes, or `None` when the scanner responds with 404
/// (no more pages) or a terminal job state is detected.
/// Retries up to `max_retries` times on 503 Service Unavailable.
pub async fn get_next_document(
    client: &ScannerClient,
    job_path: &str,
    max_retries: u32,
) -> Result<Option<Vec<u8>>> {
    let path = format!("{}/NextDocument", job_path);
    let mut attempt = 0u32;

    loop {
        let resp = client.get(&path).await?;
        let status = resp.status();

        match status.as_u16() {
            200 => {
                let bytes = resp.bytes().await?.to_vec();
                return Ok(Some(bytes));
            }
            404 | 410 => {
                // No more pages.
                return Ok(None);
            }
            503 => {
                attempt += 1;
                if attempt >= max_retries {
                    bail!("GET {path} returned 503 after {max_retries} retries");
                }
                let wait = std::time::Duration::from_millis(500 * u64::from(attempt));
                debug!("503 on NextDocument, retry {attempt}/{max_retries} in {wait:?}");
                tokio::time::sleep(wait).await;
            }
            other => {
                bail!("GET {path} returned unexpected status {other}");
            }
        }
    }
}

/// GET `{job_path}/ScanImageInfo`
pub async fn get_scan_image_info(
    client: &ScannerClient,
    job_path: &str,
) -> Result<ScanImageInfo> {
    let path = format!("{}/ScanImageInfo", job_path);
    let resp = client.get(&path).await?;
    let status = resp.status();
    if !status.is_success() {
        bail!("GET {path} returned {status}");
    }
    let xml = resp.text().await?;
    parse_scan_image_info(&xml)
}
