use anyhow::{bail, Result};
use printpdf::{Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, PdfWarnMsg, Pt, RawImage, XObjectTransform};

use crate::scanning::job::ScannedPage;

/// Points per millimetre (PDF user units = 1/72 inch; 1 mm = 72/25.4 pt).
const PT_PER_MM: f32 = 72.0 / 25.4;

/// Assemble a list of JPEG pages into a single multi-page PDF.
/// Returns the raw PDF bytes.
pub fn pages_to_pdf(pages: &[ScannedPage]) -> Result<Vec<u8>> {
    if pages.is_empty() {
        bail!("cannot create PDF from zero pages");
    }

    let mut doc = PdfDocument::new("Scan");
    let mut pdf_pages: Vec<PdfPage> = Vec::with_capacity(pages.len());
    let mut warnings: Vec<PdfWarnMsg> = Vec::new();

    for page in pages {
        let w_mm = page.width as f32 / page.resolution as f32 * 25.4;
        let h_mm = page.height as f32 / page.resolution as f32 * 25.4;

        // Decode JPEG into a RawImage that printpdf can embed.
        let raw_image = RawImage::decode_from_bytes(&page.jpeg_bytes, &mut warnings)
            .map_err(|e| anyhow::anyhow!("failed to decode JPEG: {e}"))?;

        let image_id = doc.add_image(&raw_image);

        // In PDF, an unscaled image XObject is 1×1 pt. We scale it to fill the
        // page: scale factors equal the page dimensions in points.
        let w_pt = w_mm * PT_PER_MM;
        let h_pt = h_mm * PT_PER_MM;

        let transform = XObjectTransform {
            translate_x: Some(Pt(0.0)),
            translate_y: Some(Pt(0.0)),
            scale_x: Some(w_pt),
            scale_y: Some(h_pt),
            ..Default::default()
        };

        let ops = vec![Op::UseXobject {
            id: image_id,
            transform,
        }];

        pdf_pages.push(PdfPage::new(Mm(w_mm), Mm(h_mm), ops));
    }

    doc.with_pages(pdf_pages);

    let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
    Ok(bytes)
}

