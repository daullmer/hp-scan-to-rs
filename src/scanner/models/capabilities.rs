use anyhow::{bail, Result};

/// Scanner capabilities parsed from `/eSCL/ScannerCapabilities`.
#[derive(Debug, Clone)]
pub struct ScannerCapabilities {
    /// Platen (flatbed glass) max width in 1/300-inch units.
    pub platen_max_width: Option<u32>,
    /// Platen max height in 1/300-inch units.
    pub platen_max_height: Option<u32>,
    /// ADF simplex max width in 1/300-inch units.
    pub adf_max_width: Option<u32>,
    /// ADF simplex max height in 1/300-inch units.
    pub adf_max_height: Option<u32>,
    /// ADF duplex max width in 1/300-inch units.
    pub adf_duplex_max_width: Option<u32>,
    /// ADF duplex max height in 1/300-inch units.
    pub adf_duplex_max_height: Option<u32>,
    pub has_adf: bool,
    pub has_adf_duplex: bool,
}

/// Parse `/eSCL/ScannerCapabilities` XML response.
pub fn parse_capabilities(xml: &str) -> Result<ScannerCapabilities> {
    let doc = roxmltree::Document::parse(xml)?;
    let root = doc.root_element();

    let mut platen_max_width = None;
    let mut platen_max_height = None;
    let mut adf_max_width = None;
    let mut adf_max_height = None;
    let mut adf_duplex_max_width = None;
    let mut adf_duplex_max_height = None;
    let mut has_adf = false;
    let mut has_adf_duplex = false;

    // Walk the tree looking for Platen and Adf sections.
    for node in root.descendants() {
        let local = node.tag_name().name();
        match local {
            "Platen" => {
                for child in node.descendants() {
                    match child.tag_name().name() {
                        "MaxWidth" => platen_max_width = child.text().and_then(|t| t.parse().ok()),
                        "MaxHeight" => platen_max_height = child.text().and_then(|t| t.parse().ok()),
                        _ => {}
                    }
                }
            }
            "Adf" => {
                has_adf = true;
                for child in node.descendants() {
                    match child.tag_name().name() {
                        "MaxWidth" if is_under(child, "AdfSimplex") => {
                            adf_max_width = child.text().and_then(|t| t.parse().ok());
                        }
                        "MaxHeight" if is_under(child, "AdfSimplex") => {
                            adf_max_height = child.text().and_then(|t| t.parse().ok());
                        }
                        "MaxWidth" if is_under(child, "AdfDuplex") => {
                            adf_duplex_max_width = child.text().and_then(|t| t.parse().ok());
                            has_adf_duplex = true;
                        }
                        "MaxHeight" if is_under(child, "AdfDuplex") => {
                            adf_duplex_max_height = child.text().and_then(|t| t.parse().ok());
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    if platen_max_width.is_none() && adf_max_width.is_none() {
        bail!("could not parse scanner capabilities: no platen or ADF dimensions found");
    }

    Ok(ScannerCapabilities {
        platen_max_width,
        platen_max_height,
        adf_max_width,
        adf_max_height,
        adf_duplex_max_width,
        adf_duplex_max_height,
        has_adf,
        has_adf_duplex,
    })
}

/// Check whether a node has an ancestor with the given local name.
fn is_under(node: roxmltree::Node, ancestor_name: &str) -> bool {
    let mut current = node.parent();
    while let Some(p) = current {
        if p.tag_name().name() == ancestor_name {
            return true;
        }
        current = p.parent();
    }
    false
}
