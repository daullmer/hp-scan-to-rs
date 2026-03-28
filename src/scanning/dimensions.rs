use crate::config::{DestinationConfig, PaperSize};
use crate::scanner::models::capabilities::ScannerCapabilities;
use crate::scanner::models::scan_settings::InputSource;

/// Scan region in eSCL 1/300-inch units.
#[derive(Debug, Clone, Copy)]
pub struct ScanRegion {
    pub width: u32,
    pub height: u32,
}

/// Millimetres-per-inch constant.
const MM_PER_INCH: f64 = 25.4;
/// eSCL uses 1/300 inch as its base unit.
const ESCL_UNITS_PER_INCH: f64 = 300.0;

/// Convert millimetres to eSCL 1/300-inch units.
fn mm_to_escl(mm: f64) -> u32 {
    ((mm / MM_PER_INCH) * ESCL_UNITS_PER_INCH).round() as u32
}

/// Standard paper dimensions in millimetres (width × height, portrait).
fn paper_size_mm(size: &PaperSize) -> Option<(f64, f64)> {
    match size {
        PaperSize::A3 => Some((297.0, 420.0)),
        PaperSize::A4 => Some((210.0, 297.0)),
        PaperSize::A5 => Some((148.0, 210.0)),
        PaperSize::B5 => Some((176.0, 250.0)),
        PaperSize::Letter => Some((215.9, 279.4)),
        PaperSize::Legal => Some((215.9, 355.6)),
        PaperSize::Max => None, // use device maximum
    }
}

/// Resolve the scan region for the given config and scanner capabilities.
///
/// If `paper_size` is `Max`, the device's reported maximum for `input_source`
/// is used. Otherwise the paper preset is converted to eSCL units and clamped
/// to the device maximum.
pub fn resolve_scan_region(
    config: &DestinationConfig,
    input_source: InputSource,
    caps: &ScannerCapabilities,
    duplex: bool,
) -> ScanRegion {
    let (device_max_w, device_max_h) = device_max(caps, input_source, duplex);

    let (raw_w, raw_h) = match paper_size_mm(&config.paper_size) {
        Some((w_mm, h_mm)) => (mm_to_escl(w_mm), mm_to_escl(h_mm)),
        None => {
            // "max" — use device capability directly.
            return ScanRegion {
                width: device_max_w.unwrap_or(2550),
                height: device_max_h.unwrap_or(3300),
            };
        }
    };

    ScanRegion {
        width: clamp(raw_w, device_max_w),
        height: clamp(raw_h, device_max_h),
    }
}

fn clamp(value: u32, max: Option<u32>) -> u32 {
    match max {
        Some(m) => value.min(m).max(1),
        None => value.max(1),
    }
}

fn device_max(
    caps: &ScannerCapabilities,
    source: InputSource,
    duplex: bool,
) -> (Option<u32>, Option<u32>) {
    match source {
        InputSource::Platen => (caps.platen_max_width, caps.platen_max_height),
        InputSource::Adf => {
            if duplex {
                (caps.adf_duplex_max_width, caps.adf_duplex_max_height)
            } else {
                (caps.adf_max_width, caps.adf_max_height)
            }
        }
    }
}
