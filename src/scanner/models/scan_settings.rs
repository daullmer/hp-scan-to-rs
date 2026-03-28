use crate::config::{ColorMode, DestinationConfig};
use crate::scanning::dimensions::ScanRegion;

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

    let duplex_element = if config.duplex && input_source == InputSource::Adf {
        "\n  <scan:Duplex>true</scan:Duplex>"
    } else {
        ""
    };

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scan:ScanSettings xmlns:scan="http://schemas.hp.com/imaging/escl/2011/05/03" xmlns:pwg="http://www.pwg.org/schemas/2010/12/sm">
  <pwg:Version>2.0</pwg:Version>
  <scan:Intent>TextAndGraphic</scan:Intent>
  <pwg:ScanRegions pwg:MustHonor="true">
    <pwg:ScanRegion>
      <pwg:ContentRegionUnits>escl:ThreeHundredthsOfInches</pwg:ContentRegionUnits>
      <pwg:Width>{}</pwg:Width>
      <pwg:Height>{}</pwg:Height>
      <pwg:XOffset>0</pwg:XOffset>
      <pwg:YOffset>0</pwg:YOffset>
    </pwg:ScanRegion>
  </pwg:ScanRegions>
  <pwg:DocumentFormat>image/jpeg</pwg:DocumentFormat>
  <pwg:InputSource>{}</pwg:InputSource>
  <scan:ColorMode>{}</scan:ColorMode>
  <scan:XResolution>{}</scan:XResolution>
  <scan:YResolution>{}</scan:YResolution>{}
</scan:ScanSettings>"#,
        region.width,
        region.height,
        source,
        color_mode,
        config.resolution,
        config.resolution,
        duplex_element,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSource {
    Platen,
    Adf,
}
