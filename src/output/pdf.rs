use anyhow::{bail, Result};

use crate::scanning::job::ScannedPage;

/// Assemble a list of JPEG pages into a single multi-page PDF.
/// Embeds the JPEG bytes directly (DCTDecode) — no decode/re-encode cycle.
pub fn pages_to_pdf(pages: &[ScannedPage]) -> Result<Vec<u8>> {
    use lopdf::{Dictionary, Document, Object, Stream};

    if pages.is_empty() {
        bail!("cannot create PDF from zero pages");
    }

    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut page_ids = Vec::with_capacity(pages.len());

    for page in pages {
        let w_pt = page.width as f64 / page.resolution as f64 * 72.0;
        let h_pt = page.height as f64 / page.resolution as f64 * 72.0;

        // Image XObject: embed raw JPEG bytes with DCTDecode.
        let img_dict = Dictionary::from_iter(vec![
            ("Type", Object::Name(b"XObject".to_vec())),
            ("Subtype", Object::Name(b"Image".to_vec())),
            ("Width", Object::Integer(page.width as i64)),
            ("Height", Object::Integer(page.height as i64)),
            ("ColorSpace", Object::Name(b"DeviceRGB".to_vec())),
            ("BitsPerComponent", Object::Integer(8)),
            ("Filter", Object::Name(b"DCTDecode".to_vec())),
        ]);
        let mut img_stream = Stream::new(img_dict, page.jpeg_bytes.clone());
        img_stream.allows_compression = false; // already JPEG-compressed
        let img_id = doc.add_object(img_stream);

        // Resources: map /Im1 to our image XObject.
        let xobject_dict = Dictionary::from_iter(vec![("Im1", Object::Reference(img_id))]);
        let resources = Dictionary::from_iter(vec![(
            "XObject",
            Object::Dictionary(xobject_dict),
        )]);

        // Content stream: draw the image scaled to the page size.
        let content = format!(
            "q {w:.4} 0 0 {h:.4} 0 0 cm /Im1 Do Q",
            w = w_pt,
            h = h_pt,
        );
        let content_id = doc.add_object(Stream::new(Dictionary::new(), content.into_bytes()));

        // Page object.
        let page_dict = Dictionary::from_iter(vec![
            ("Type", Object::Name(b"Page".to_vec())),
            ("Parent", Object::Reference(pages_id)),
            (
                "MediaBox",
                Object::Array(vec![
                    Object::Integer(0),
                    Object::Integer(0),
                    Object::Real(w_pt as f32),
                    Object::Real(h_pt as f32),
                ]),
            ),
            ("Resources", Object::Dictionary(resources)),
            ("Contents", Object::Reference(content_id)),
        ]);
        let page_id = doc.add_object(page_dict);
        page_ids.push(page_id);
    }

    // Pages node.
    let kids: Vec<Object> = page_ids.iter().map(|id| Object::Reference(*id)).collect();
    let pages_dict = Dictionary::from_iter(vec![
        ("Type", Object::Name(b"Pages".to_vec())),
        ("Kids", Object::Array(kids)),
        ("Count", Object::Integer(page_ids.len() as i64)),
    ]);
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

    // Catalog.
    let catalog = Dictionary::from_iter(vec![
        ("Type", Object::Name(b"Catalog".to_vec())),
        ("Pages", Object::Reference(pages_id)),
    ]);
    let catalog_id = doc.add_object(catalog);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let mut buf = Vec::new();
    doc.save_to(&mut buf)
        .map_err(|e| anyhow::anyhow!("failed to write PDF: {e}"))?;
    Ok(buf)
}

