/// Fix JPEG dimensions when the scanner uses a DNL (Define Number of Lines)
/// marker instead of embedding the height in the SOF0 frame header.
///
/// Some HP ADF scanners write the SOF0 height as 0 and emit a DNL marker
/// (0xFFDC) at the end of the first scan. We patch the SOF0 height in-place.
///
/// If the JPEG already has a non-zero SOF0 height, or if `actual_height` is
/// provided (from `ScanImageInfo`), that value is used to force-correct the
/// header so consumers see consistent metadata.
pub fn fix_jpeg_dimensions(mut data: Vec<u8>, actual_width: u32, actual_height: u32) -> Vec<u8> {
    // Locate the SOF0 marker (0xFFC0).
    if let Some(sof0_offset) = find_marker(&data, 0xFF, 0xC0) {
        // SOF0 layout (after the 0xFFC0 marker bytes):
        //   2 bytes: segment length
        //   1 byte:  sample precision
        //   2 bytes: image height  ← offset +5 from marker start
        //   2 bytes: image width   ← offset +7 from marker start
        let h_off = sof0_offset + 5;
        let w_off = sof0_offset + 7;

        if h_off + 1 < data.len() && w_off + 1 < data.len() {
            let existing_h = u16::from_be_bytes([data[h_off], data[h_off + 1]]) as u32;

            // Only patch if the height is 0 (DNL case) or mismatches.
            if existing_h == 0 || existing_h != actual_height {
                let h = (actual_height as u16).to_be_bytes();
                data[h_off] = h[0];
                data[h_off + 1] = h[1];
            }

            let existing_w = u16::from_be_bytes([data[w_off], data[w_off + 1]]) as u32;
            if existing_w == 0 || existing_w != actual_width {
                let w = (actual_width as u16).to_be_bytes();
                data[w_off] = w[0];
                data[w_off + 1] = w[1];
            }
        }
    }

    data
}

/// Scan `data` for a two-byte marker sequence and return its offset.
fn find_marker(data: &[u8], b0: u8, b1: u8) -> Option<usize> {
    data.windows(2)
        .position(|w| w[0] == b0 && w[1] == b1)
}
