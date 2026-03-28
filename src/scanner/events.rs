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
    /// Resource URI of the WalkupScanToCompEvent endpoint to poll for
    /// the actual scan request (HostSelected → ScanRequested).
    pub comp_event_uri: Option<String>,
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
///
/// The scanner returns `<ev:Event>` nodes.  We look for those whose
/// `<dd:UnqualifiedEventCategory>` is `"ScanEvent"`, then extract the
/// destination and compEvent URIs from the `<ev:Payload>` children.
fn parse_scan_events(xml: &str) -> Vec<ScanEvent> {
    let doc: roxmltree::Document = match roxmltree::Document::parse(xml) {
        Ok(d) => d,
        Err(_) => return vec![],
    };

    let mut events = Vec::new();

    for node in doc.root_element().children() {
        if node.tag_name().name() != "Event" {
            continue;
        }

        // Check UnqualifiedEventCategory == "ScanEvent".
        let is_scan_event = node.children().any(|c| {
            c.tag_name().name() == "UnqualifiedEventCategory"
                && c.text().map_or(false, |t| t == "ScanEvent")
        });
        if !is_scan_event {
            continue;
        }

        let mut destination_uri = None;
        let mut comp_event_uri = None;

        // Each <ev:Payload> has a ResourceURI and ResourceType.
        for payload in node.children().filter(|c| c.tag_name().name() == "Payload") {
            let mut uri = None;
            let mut rtype = None;
            for child in payload.children() {
                match child.tag_name().name() {
                    "ResourceURI" => uri = child.text().map(|t| t.to_string()),
                    "ResourceType" => rtype = child.text().map(|t| t.to_string()),
                    _ => {}
                }
            }
            if let (Some(u), Some(t)) = (uri, rtype) {
                if t.contains("Destination") {
                    destination_uri = Some(u);
                } else if t.contains("CompEvent") {
                    comp_event_uri = Some(u);
                }
            }
        }

        if let Some(dest_uri) = destination_uri {
            events.push(ScanEvent {
                destination_uri: dest_uri,
                comp_event_uri,
            });
        }
    }

    events
}
