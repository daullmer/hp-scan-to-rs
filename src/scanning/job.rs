use anyhow::{bail, Result};
use tracing::{debug, info, warn};

use crate::config::DestinationConfig;
use crate::scanner::client::ScannerClient;
use crate::scanner::escl::{
    create_scan_job, get_capabilities, get_next_document, get_scan_image_info, get_scanner_status,
};
use crate::scanner::models::scan_settings::{build_scan_settings, InputSource};
use crate::scanner::models::scanner_status::{
    AdfState, JobState, ScannerState, ScannerStatus,
};
use crate::scanning::dimensions::resolve_scan_region;
use crate::scanning::pages::fix_jpeg_dimensions;

const MAX_503_RETRIES: u32 = 30;

/// A single scanned page ready for output processing.
#[derive(Debug)]
pub struct ScannedPage {
    /// Raw JPEG bytes with corrected dimension headers.
    pub jpeg_bytes: Vec<u8>,
    /// Actual width in pixels (from ScanImageInfo).
    pub width: u32,
    /// Actual height in pixels (from ScanImageInfo).
    pub height: u32,
    /// The DPI used for this scan (same as config.resolution).
    pub resolution: u32,
}

/// Execute a full scan job for the given destination config and return all
/// scanned pages as JPEG data.
pub async fn execute_scan(
    client: &ScannerClient,
    config: &DestinationConfig,
) -> Result<Vec<ScannedPage>> {
    let caps = get_capabilities(client).await?;

    // Wait for the scanner to become idle before submitting the job.
    let status = wait_for_idle(client).await?;

    // Choose input source: use ADF when paper is loaded and duplex is enabled
    // in config, or when paper is loaded and there's an ADF.
    let input_source = if caps.has_adf && status.adf_state == AdfState::Loaded {
        info!("ADF paper detected — using feeder");
        InputSource::Adf
    } else {
        info!("using platen");
        InputSource::Platen
    };

    let duplex = config.duplex && input_source == InputSource::Adf && caps.has_adf_duplex;
    let region = resolve_scan_region(config, input_source, &caps, duplex);
    debug!("scan region: {}×{} (1/300-inch units)", region.width, region.height);

    let settings_xml = build_scan_settings(config, &region, input_source);
    debug!("scan settings:\n{settings_xml}");

    let job_path = create_scan_job(client, settings_xml).await?;
    info!("scan job started: {job_path}");

    let mut pages = Vec::new();
    let mut page_num = 0u32;

    loop {
        // Small pause between page polls to avoid hammering the scanner.
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        match get_next_document(client, &job_path, MAX_503_RETRIES).await? {
            None => {
                debug!("NextDocument returned 404 — no more pages");
                break;
            }
            Some(jpeg_bytes) => {
                page_num += 1;

                // Fetch actual dimensions for this page.
                let info = get_scan_image_info(client, &job_path).await.unwrap_or_else(|e| {
                    warn!("could not fetch ScanImageInfo: {e} — using region dimensions");
                    crate::scanner::models::scanner_status::ScanImageInfo {
                        actual_width: region.width,
                        actual_height: region.height,
                    }
                });

                let fixed = fix_jpeg_dimensions(jpeg_bytes, info.actual_width, info.actual_height);
                info!("page {page_num}: {}×{} px", info.actual_width, info.actual_height);

                pages.push(ScannedPage {
                    jpeg_bytes: fixed,
                    width: info.actual_width,
                    height: info.actual_height,
                    resolution: config.resolution,
                });
            }
        }

        // Check job status — stop looping when the job is done.
        let status = get_scanner_status(client).await?;
        let job_done = status.jobs.iter().any(|j| {
            j.job_uri.contains(&job_path)
                && matches!(j.job_state, JobState::Completed | JobState::Canceled)
        });
        // Also stop if the scanner says it's idle and we have at least one page.
        let scanner_idle = matches!(status.state, ScannerState::Idle);

        if job_done || (scanner_idle && !pages.is_empty()) {
            debug!("job done (job_done={job_done}, scanner_idle={scanner_idle})");
            break;
        }
    }

    if pages.is_empty() {
        bail!("scan job completed but produced no pages");
    }

    info!("scan complete: {} page(s)", pages.len());
    Ok(pages)
}

/// Poll the scanner status until it reports `Idle`, up to ~30 seconds.
async fn wait_for_idle(client: &ScannerClient) -> Result<ScannerStatus> {
    const MAX_ATTEMPTS: u32 = 30;
    for attempt in 1..=MAX_ATTEMPTS {
        let status = get_scanner_status(client).await?;
        if status.state == ScannerState::Idle {
            return Ok(status);
        }
        debug!(
            "scanner state: {:?} — waiting for Idle (attempt {attempt}/{MAX_ATTEMPTS})",
            status.state
        );
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    // Return last status even if not idle — let the caller decide.
    get_scanner_status(client).await
}
