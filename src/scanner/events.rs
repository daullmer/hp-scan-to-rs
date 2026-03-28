use anyhow::{bail, Result};
use tracing::debug;

use crate::scanner::client::ScannerClient;

const EVENT_TABLE_PATH: &str = "/EventMgmt/EventTable";
/// Long-poll timeout in deciseconds (120 s).
const POLL_TIMEOUT_DS: u32 = 1200;

/// An event indicating the user selected a scan destination.
#[derive(Debug, Clone)]
pub struct ScanEvent {
    /// Resource URI of the destination the user selected, e.g.
    /// `/WalkupScanToComp/WalkupScanToCompDestinations/1`
    pub destination_uri: String,
}

/// Result of one poll operation.
pub enum PollResult {
    /// No change since the last poll (304 Not Modified).
    NoChange,
    /// One or more scan events detected.
    Events { etag: String, events: Vec<ScanEvent> },
    /// Response received but contained no scan events.
    Empty { etag: String },
}

/// Poll `/EventMgmt/EventTable` once.
///
/// Pass the ETag from the previous response as `etag` to enable long-polling
/// (the scanner holds the connection open until an event occurs or the timeout
/// elapses). Pass `None` on the first call.
pub async fn poll_events(client: &ScannerClient, etag: Option<&str>) -> Result<PollResult> {
    let path = format!("{}?timeout={}", EVENT_TABLE_PATH, POLL_TIMEOUT_DS);
    let resp = client.get_conditional(&path, etag).await?;
    let status = resp.status();

    if status.as_u16() == 304 {
        return Ok(PollResult::NoChange);
    }

    if !status.is_success() {
        bail!("EventTable poll returned {status}");
    }

    let new_etag = resp
        .headers()
        .get("ETag")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let xml = resp.text().await?;
    debug!("EventTable response (etag={new_etag:?}):\n{xml}");

    let events = parse_scan_events(&xml);
    if events.is_empty() {
        Ok(PollResult::Empty { etag: new_etag })
    } else {
        Ok(PollResult::Events {
            etag: new_etag,
            events,
        })
    }
}

/// Extract scan destination selection events from the EventTable XML.
fn parse_scan_events(xml: &str) -> Vec<ScanEvent> {
    let doc: roxmltree::Document = match roxmltree::Document::parse(xml) {
        Ok(d) => d,
        Err(_) => return vec![],
    };

    let mut events = Vec::new();

    for node in doc.root_element().descendants() {
        // Look for <ScanEvent> or <WalkupScanToCompEvent> nodes that signal a
        // destination was selected.
        let name = node.tag_name().name();
        if name == "ScanEvent" || name == "WalkupScanToCompEvent" {
            let mut destination_uri = String::new();
            let mut event_type = String::new();

            for child in node.descendants() {
                match child.tag_name().name() {
                    "WalkupScanToCompDestinationURI" | "DestinationURI" => {
                        destination_uri = child.text().unwrap_or("").to_string();
                    }
                    "WalkupScanToCompEventType" | "ScanEventType" => {
                        event_type = child.text().unwrap_or("").to_string();
                    }
                    _ => {}
                }
            }

            // We care about HostSelected (user touched the destination on LCD)
            // and ScanRequested (scan button pressed).
            let relevant = matches!(
                event_type.as_str(),
                "HostSelected" | "ScanRequested" | "ScanNewPageRequested" | ""
            );

            if relevant && !destination_uri.is_empty() {
                events.push(ScanEvent { destination_uri });
            }
        }
    }

    events
}
