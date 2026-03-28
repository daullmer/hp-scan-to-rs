use anyhow::Result;

/// Build the XML body for a `POST /WalkupScanToComp/WalkupScanToCompDestinations` request.
///
/// `label` is used as both `Name` (shown on scanner LCD) and `Hostname`
/// (the scanner groups destinations by hostname, so each label must have
/// its own unique hostname to appear as a separate entry).
pub fn build_destination_xml(label: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<WalkupScanToCompDestination xmlns="http://www.hp.com/schemas/imaging/con/ledm/walkupscan/2010/09/28" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://www.hp.com/schemas/imaging/con/ledm/walkupscan/2010/09/28 WalkupScanToComp.xsd">
	<Hostname xmlns="http://www.hp.com/schemas/imaging/con/dictionaries/2009/04/06">{0}</Hostname>
	<Name xmlns="http://www.hp.com/schemas/imaging/con/dictionaries/1.0/">{0}</Name>
	<LinkType>Network</LinkType>
</WalkupScanToCompDestination>"#,
        escape_xml(label)
    )
}

/// Escape characters that are not safe in XML text content.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// A destination entry returned by the scanner.
#[derive(Debug, Clone)]
pub struct RegisteredDestination {
    /// Full resource URI, e.g.
    /// `/WalkupScanToComp/WalkupScanToCompDestinations/1`
    pub resource_uri: String,
    pub name: String,
}

/// Parse the list response from
/// `GET /WalkupScanToComp/WalkupScanToCompDestinations`.
pub fn parse_destinations_list(xml: &str) -> Result<Vec<RegisteredDestination>> {
    let doc = roxmltree::Document::parse(xml)?;
    let mut destinations = Vec::new();

    for node in doc.root_element().descendants() {
        if node.tag_name().name() == "WalkupScanToCompDestination" {
            let mut resource_uri = String::new();
            let mut name = String::new();

            for child in node.children() {
                match child.tag_name().name() {
                    "ResourceURI" => resource_uri = child.text().unwrap_or("").to_string(),
                    "Name" => name = child.text().unwrap_or("").to_string(),
                    _ => {}
                }
            }

            if !resource_uri.is_empty() {
                destinations.push(RegisteredDestination {
                    resource_uri,
                    name,
                });
            }
        }
    }

    Ok(destinations)
}

/// Parse the max number of destinations from
/// `GET /WalkupScanToComp/WalkupScanToCompCaps`.
pub fn parse_max_destinations(xml: &str) -> Option<u32> {
    let doc = roxmltree::Document::parse(xml).ok()?;
    for node in doc.root_element().descendants() {
        if node.tag_name().name() == "MaxNumDestinations" {
            return node.text().and_then(|t| t.parse().ok());
        }
    }
    None
}
