use anyhow::Result;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::destinations::DestinationMap;
use crate::output::dispatch;
use crate::scanner::client::ScannerClient;
use crate::scanner::events::{poll_events, PollResult};
use crate::scanner::walkup::{get_comp_event_type, CompEventType};
use crate::scanning::job::execute_scan;

/// Run the main event loop.
///
/// Polls the scanner's EventTable indefinitely. When the user selects one of
/// our registered destinations on the scanner LCD, a scan job is executed and
/// the result is dispatched to the configured output.
///
/// Returns when `shutdown` is triggered (via SIGINT / SIGTERM).
pub async fn run(
    client: &ScannerClient,
    config: &Config,
    dest_map: &DestinationMap,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<()> {
    let mut etag: Option<String> = None;
    let mut consecutive_errors = 0u32;
    const MAX_ERRORS: u32 = 10;

    info!("event loop started — waiting for scanner events");

    loop {
        // Check for shutdown signal without blocking.
        if *shutdown.borrow() {
            info!("shutdown requested — stopping event loop");
            break;
        }

        // Poll the event table.
        let poll_result = tokio::select! {
            result = poll_events(client, etag.as_deref()) => result,
            _ = shutdown.changed() => {
                info!("shutdown requested — stopping event loop");
                break;
            }
        };

        match poll_result {
            Err(e) => {
                consecutive_errors += 1;
                warn!("event table poll error ({consecutive_errors}/{MAX_ERRORS}): {e}");
                if consecutive_errors >= MAX_ERRORS {
                    error!("too many consecutive scanner errors — giving up");
                    return Err(e);
                }
                // Back off before retrying.
                let wait = std::time::Duration::from_secs(2 * u64::from(consecutive_errors));
                tokio::time::sleep(wait).await;
                continue;
            }
            Ok(PollResult::NoChange) => {
                consecutive_errors = 0;
                continue;
            }
            Ok(PollResult::Empty { etag: new_etag }) => {
                consecutive_errors = 0;
                etag = Some(new_etag);
                continue;
            }
            Ok(PollResult::Events {
                etag: new_etag,
                events,
            }) => {
                consecutive_errors = 0;
                etag = Some(new_etag);

                for event in events {
                    let Some(&dest_idx) = dest_map.get(&event.destination_uri) else {
                        // Event is for a destination we don't own — ignore.
                        debug!("ignoring event for unknown destination {}", event.destination_uri);
                        continue;
                    };

                    let dest_config = &config.destinations[dest_idx];
                    info!("destination {:?} selected on scanner", dest_config.label);

                    // Wait for the user to press the scan button.
                    if let Some(ref comp_uri) = event.comp_event_uri {
                        if !wait_for_scan_request(client, comp_uri).await {
                            info!("scan cancelled or timed out for {:?}", dest_config.label);
                            continue;
                        }
                    }

                    info!("scan started for {:?}", dest_config.label);

                    match execute_scan(client, dest_config).await {
                        Err(e) => {
                            error!("scan failed for {:?}: {e}", dest_config.label);
                        }
                        Ok(pages) => {
                            if let Err(e) = dispatch(dest_config, &pages).await {
                                error!(
                                    "output dispatch failed for {:?}: {e}",
                                    dest_config.label
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Poll the WalkupScanToCompEvent endpoint until the user presses scan
/// or the attempt times out / is cancelled.
async fn wait_for_scan_request(client: &ScannerClient, comp_event_uri: &str) -> bool {
    const MAX_ATTEMPTS: u32 = 50;
    for attempt in 1..=MAX_ATTEMPTS {
        match get_comp_event_type(client, comp_event_uri).await {
            Ok(CompEventType::ScanRequested | CompEventType::ScanNewPageRequested) => {
                return true;
            }
            Ok(CompEventType::HostSelected) => {
                debug!("waiting for scan button (attempt {attempt}/{MAX_ATTEMPTS})");
            }
            Ok(other) => {
                debug!("comp event type: {other:?} — scan not proceeding");
                return false;
            }
            Err(e) => {
                warn!("failed to get comp event: {e}");
                return false;
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    warn!("timed out waiting for scan button");
    false
}
