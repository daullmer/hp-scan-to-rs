/// Fix JPEG dimensions when the scanner uses a DNL (Define Number of Lines)
/// marker instead of embedding the height in the SOF0 frame header.
///
/// Some HP ADF scanners write the SOF0 height as 0 and emit a DNL marker
/// (0xFFDC) at the end of the first scan. We patch the SOF0 height in-place.
///
/// Valid dimensions in a scanner-provided JPEG are never changed. The fallback
/// dimensions are used only when a DNL-style JPEG has zeros in its SOF0 header.
pub fn fix_jpeg_dimensions(
    mut data: Vec<u8>,
    fallback_width: u32,
    fallback_height: u32,
) -> Vec<u8> {
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

            if existing_h == 0 {
                let h = (fallback_height as u16).to_be_bytes();
                data[h_off] = h[0];
                data[h_off + 1] = h[1];
            }

            let existing_w = u16::from_be_bytes([data[w_off], data[w_off + 1]]) as u32;
            if existing_w == 0 {
                let w = (fallback_width as u16).to_be_bytes();
                data[w_off] = w[0];
                data[w_off + 1] = w[1];
            }
        }
    }

    data
}

/// Return the dimensions declared in a JPEG SOF0 header.
pub fn jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    let sof0_offset = find_marker(data, 0xFF, 0xC0)?;
    let h_off = sof0_offset + 5;
    let w_off = sof0_offset + 7;
    if h_off + 1 >= data.len() || w_off + 1 >= data.len() {
        return None;
    }

    let height = u16::from_be_bytes([data[h_off], data[h_off + 1]]) as u32;
    let width = u16::from_be_bytes([data[w_off], data[w_off + 1]]) as u32;
    (width > 0 && height > 0).then_some((width, height))
}

/// Scan `data` for a two-byte marker sequence and return its offset.
fn find_marker(data: &[u8], b0: u8, b1: u8) -> Option<usize> {
    data.windows(2).position(|w| w[0] == b0 && w[1] == b1)
}

#[cfg(test)]
mod tests {
    use super::{fix_jpeg_dimensions, jpeg_dimensions};

    fn jpeg_with_dimensions(width: u16, height: u16) -> Vec<u8> {
        vec![
            0xFF,
            0xD8,
            0xFF,
            0xC0,
            0,
            17,
            8,
            (height >> 8) as u8,
            height as u8,
            (width >> 8) as u8,
            width as u8,
            3,
        ]
    }

    #[test]
    fn keeps_valid_scanner_dimensions() {
        let jpeg = jpeg_with_dimensions(1653, 2338);
        let fixed = fix_jpeg_dimensions(jpeg, 2480, 3508);

        assert_eq!(jpeg_dimensions(&fixed), Some((1653, 2338)));
    }

    #[test]
    fn fills_missing_dimensions() {
        let jpeg = jpeg_with_dimensions(0, 0);
        let fixed = fix_jpeg_dimensions(jpeg, 1653, 2338);

        assert_eq!(jpeg_dimensions(&fixed), Some((1653, 2338)));
    }
}
