use serde::Serialize;

use crate::config::{ColorMode, DestinationConfig};
use crate::scanning::dimensions::ScanRegion;

const NS_SCAN: &str = "http://schemas.hp.com/imaging/escl/2011/05/03";
const NS_PWG: &str = "http://www.pwg.org/schemas/2010/12/sm";

#[derive(Serialize)]
#[serde(rename = "scan:ScanSettings")]
struct ScanSettingsXml {
    #[serde(rename = "@xmlns:scan")]
    xmlns_scan: &'static str,
    #[serde(rename = "@xmlns:pwg")]
    xmlns_pwg: &'static str,
    #[serde(rename = "pwg:Version")]
    version: &'static str,
    #[serde(rename = "scan:Intent")]
    intent: &'static str,
    #[serde(rename = "pwg:ScanRegions")]
    scan_regions: ScanRegionsXml,
    #[serde(rename = "pwg:DocumentFormat")]
    document_format: &'static str,
    #[serde(rename = "pwg:InputSource")]
    input_source: &'static str,
    #[serde(rename = "scan:ColorMode")]
    color_mode: &'static str,
    #[serde(rename = "scan:XResolution")]
    x_resolution: u32,
    #[serde(rename = "scan:YResolution")]
    y_resolution: u32,
    #[serde(rename = "scan:Duplex")]
    duplex: bool,
}

#[derive(Serialize)]
struct ScanRegionsXml {
    #[serde(rename = "@pwg:MustHonor")]
    must_honor: &'static str,
    #[serde(rename = "pwg:ScanRegion")]
    scan_region: ScanRegionXml,
}

#[derive(Serialize)]
struct ScanRegionXml {
    #[serde(rename = "pwg:ContentRegionUnits")]
    content_region_units: &'static str,
    #[serde(rename = "pwg:Width")]
    width: u32,
    #[serde(rename = "pwg:Height")]
    height: u32,
    #[serde(rename = "pwg:XOffset")]
    x_offset: u32,
    #[serde(rename = "pwg:YOffset")]
    y_offset: u32,
}

/// Build the eSCL `ScanSettings` XML body for a scan job.
pub fn build_scan_settings(
    config: &DestinationConfig,
    region: &ScanRegion,
    input_source: InputSource,
) -> String {
    let color_mode = match config.color_mode {
        ColorMode::Color => "RGB24",
        ColorMode::Gray => "Grayscale8",
        ColorMode::Bw => "BlackAndWhite1",
    };

    let source = match input_source {
        InputSource::Platen => "Platen",
        InputSource::Adf => "Feeder",
    };

    let duplex = config.duplex && input_source == InputSource::Adf;

    let settings = ScanSettingsXml {
        xmlns_scan: NS_SCAN,
        xmlns_pwg: NS_PWG,
        version: "2.0",
        intent: "TextAndGraphic",
        scan_regions: ScanRegionsXml {
            must_honor: "false",
            scan_region: ScanRegionXml {
                content_region_units: "escl:ThreeHundredthsOfInches",
                width: region.width,
                height: region.height,
                x_offset: 0,
                y_offset: 0,
            },
        },
        document_format: "image/jpeg",
        input_source: source,
        color_mode,
        x_resolution: config.resolution,
        y_resolution: config.resolution,
        duplex,
    };

    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let mut ser = quick_xml::se::Serializer::new(&mut xml);
    ser.indent(' ', 2);
    settings
        .serialize(ser)
        .expect("scan settings XML serialization failed");
    xml
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSource {
    Platen,
    Adf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FileFormat;
    use crate::scanning::dimensions::ScanRegion;

    fn test_config() -> DestinationConfig {
        DestinationConfig {
            label: "Test".to_string(),
            output: crate::config::OutputKind::Directory,
            directory: Some("/tmp".to_string()),
            format: Some(FileFormat::Pdf),
            to: None,
            from: None,
            subject: None,
            resolution: 300,
            color_mode: ColorMode::Color,
            paper_size: crate::config::PaperSize::A4,
            duplex: false,
            raw_jpeg_directory: None,
        }
    }

    #[test]
    fn scan_settings_xml_has_correct_namespaces() {
        let region = ScanRegion {
            width: 2480,
            height: 3508,
        };
        let xml = build_scan_settings(&test_config(), &region, InputSource::Platen);
        println!("{xml}");
        assert!(xml.contains(r#"xmlns:scan="http://schemas.hp.com/imaging/escl/2011/05/03""#));
        assert!(xml.contains(r#"xmlns:pwg="http://www.pwg.org/schemas/2010/12/sm""#));
        assert!(xml.contains("<pwg:Width>2480</pwg:Width>"));
        assert!(xml.contains("<scan:ColorMode>RGB24</scan:ColorMode>"));
        assert!(xml.contains("<pwg:InputSource>Platen</pwg:InputSource>"));
        assert!(xml.contains(r#"<pwg:ScanRegions pwg:MustHonor="false">"#));
        assert!(xml.contains("<scan:Duplex>false</scan:Duplex>"));
    }
}
