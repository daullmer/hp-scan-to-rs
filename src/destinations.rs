use anyhow::{bail, Result};
use std::collections::HashMap;
use tracing::{info, warn};

use crate::config::Config;
use crate::scanner::client::ScannerClient;
use crate::scanner::walkup::{
    delete_destination, get_max_destinations, list_destinations, register_destination,
};

/// Maps a scanner-assigned resource URI path to the index of the corresponding
/// entry in `config.destinations`.
pub type DestinationMap = HashMap<String, usize>;

/// Register all configured destinations with the scanner.
///
/// - Verifies the scanner supports at least as many destinations as configured.
/// - Removes any existing registrations whose `Name` matches a configured label
///   (stale entries from a previous run).
/// - Registers each destination fresh and returns the resource_uri → index map.
pub async fn register_all(client: &ScannerClient, config: &Config) -> Result<DestinationMap> {
    let max = get_max_destinations(client).await?;
    let num = config.destinations.len() as u32;
    if num > max {
        bail!(
            "scanner supports at most {max} destinations but config defines {num}"
        );
    }

    // Build a set of our labels for fast lookup.
    let our_labels: std::collections::HashSet<&str> =
        config.destinations.iter().map(|d| d.label.as_str()).collect();

    // Remove stale entries.
    let existing = list_destinations(client).await?;
    for dest in &existing {
        if our_labels.contains(dest.name.as_str()) {
            info!(
                "removing stale destination {:?} ({})",
                dest.name, dest.resource_uri
            );
            if let Err(e) = delete_destination(client, &dest.resource_uri).await {
                warn!("failed to delete stale destination {}: {e}", dest.resource_uri);
            }
        }
    }

    // Register all configured destinations.
    let mut map = DestinationMap::new();
    for (idx, dest) in config.destinations.iter().enumerate() {
        let uri = register_destination(client, &dest.label).await?;
        info!("registered {:?} → {uri}", dest.label);
        map.insert(uri, idx);
    }

    Ok(map)
}

/// Delete all registered destinations by their resource URIs.
pub async fn deregister_all(client: &ScannerClient, map: &DestinationMap) {
    for uri in map.keys() {
        if let Err(e) = delete_destination(client, uri).await {
            warn!("failed to delete destination {uri}: {e}");
        } else {
            info!("deregistered destination {uri}");
        }
    }
}
