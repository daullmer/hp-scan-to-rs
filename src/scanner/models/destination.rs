use anyhow::Result;
use serde::Serialize;

const NS_WALKUPSCAN: &str =
    "http://www.hp.com/schemas/imaging/con/ledm/walkupscan/2010/09/28";
const NS_XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";
const NS_SCHEMA_LOCATION: &str =
    "http://www.hp.com/schemas/imaging/con/ledm/walkupscan/2010/09/28 WalkupScanToComp.xsd";
const NS_HOSTNAME: &str = "http://www.hp.com/schemas/imaging/con/dictionaries/2009/04/06";
const NS_NAME: &str = "http://www.hp.com/schemas/imaging/con/dictionaries/1.0/";

#[derive(Serialize)]
#[serde(rename = "WalkupScanToCompDestination")]
struct WalkupScanToCompDestinationXml {
    #[serde(rename = "@xmlns")]
    xmlns: &'static str,
    #[serde(rename = "@xmlns:xsi")]
    xmlns_xsi: &'static str,
    #[serde(rename = "@xsi:schemaLocation")]
    schema_location: &'static str,
    #[serde(rename = "Hostname")]
    hostname: NsTextElement,
    #[serde(rename = "Name")]
    name: NsTextElement,
    #[serde(rename = "LinkType")]
    link_type: &'static str,
}

/// An XML element with a custom default namespace and text content.
#[derive(Serialize)]
struct NsTextElement {
    #[serde(rename = "@xmlns")]
    xmlns: &'static str,
    #[serde(rename = "$text")]
    text: String,
}

/// Build the XML body for a `POST /WalkupScanToComp/WalkupScanToCompDestinations` request.
///
/// `label` is used as both `Name` (shown on scanner LCD) and `Hostname`
/// (the scanner groups destinations by hostname, so each label must have
/// its own unique hostname to appear as a separate entry).
pub fn build_destination_xml(label: &str) -> String {
    let dest = WalkupScanToCompDestinationXml {
        xmlns: NS_WALKUPSCAN,
        xmlns_xsi: NS_XSI,
        schema_location: NS_SCHEMA_LOCATION,
        hostname: NsTextElement {
            xmlns: NS_HOSTNAME,
            text: label.to_string(),
        },
        name: NsTextElement {
            xmlns: NS_NAME,
            text: label.to_string(),
        },
        link_type: "Network",
    };

    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let mut ser = quick_xml::se::Serializer::new(&mut xml);
    ser.indent('\t', 1);
    dest.serialize(ser).expect("destination XML serialization failed");
    xml
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destination_xml_has_correct_namespaces() {
        let xml = build_destination_xml("Scan to Documents");
        println!("{xml}");
        assert!(xml.contains(r#"xmlns="http://www.hp.com/schemas/imaging/con/ledm/walkupscan/2010/09/28""#));
        assert!(xml.contains(r#"<Hostname xmlns="http://www.hp.com/schemas/imaging/con/dictionaries/2009/04/06""#));
        assert!(xml.contains(r#"<Name xmlns="http://www.hp.com/schemas/imaging/con/dictionaries/1.0/""#));
        assert!(xml.contains("Scan to Documents"));
        assert!(xml.contains("<LinkType>Network</LinkType>"));
    }

    #[test]
    fn destination_xml_escapes_special_chars() {
        let xml = build_destination_xml("Test <&> \"Name\"");
        println!("{xml}");
        assert!(!xml.contains("<&>"));
        assert!(xml.contains("&lt;&amp;&gt;"));
    }
}
