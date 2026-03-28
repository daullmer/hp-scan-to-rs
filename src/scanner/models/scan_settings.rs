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
        "<Duplex>true</Duplex>"
    } else {
        ""
    };

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ScanSettings xmlns="http://schemas.hp.com/imaging/escl/2011/05/03">
  <Version>2.62</Version>
  <Intent>Document</Intent>
  <InputSource>{}</InputSource>
  <XResolution>{}</XResolution>
  <YResolution>{}</YResolution>
  <ColorMode>{}</ColorMode>
  {}
  <ScanRegions>
    <ScanRegion>
      <Width>{}</Width>
      <Height>{}</Height>
      <ContentRegionUnits>escl:ThreeHundredthsOfInches</ContentRegionUnits>
    </ScanRegion>
  </ScanRegions>
</ScanSettings>"#,
        source,
        config.resolution,
        config.resolution,
        color_mode,
        duplex_element,
        region.width,
        region.height,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSource {
    Platen,
    Adf,
}
