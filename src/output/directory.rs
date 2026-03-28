use anyhow::{Context, Result};
use chrono::Local;
use std::path::PathBuf;
use tracing::info;

use crate::config::{DestinationConfig, FileFormat};
use crate::output::pdf::pages_to_pdf;
use crate::scanning::job::ScannedPage;

/// Write scan output to the configured directory.
pub async fn save_to_directory(
    config: &DestinationConfig,
    pages: &[ScannedPage],
) -> Result<Vec<PathBuf>> {
    let dir = resolve_dir(config.directory.as_deref().unwrap_or("."))?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("create output directory {dir:?}"))?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let format = config.format.as_ref().unwrap_or(&FileFormat::Pdf);
    let mut written = Vec::new();

    match format {
        FileFormat::Pdf => {
            let filename = format!("scan_{timestamp}.pdf");
            let path = dir.join(&filename);
            let pdf_bytes = pages_to_pdf(pages)?;
            std::fs::write(&path, &pdf_bytes)
                .with_context(|| format!("write PDF to {path:?}"))?;
            info!("saved PDF: {path:?}");
            written.push(path);
        }
        FileFormat::Jpeg => {
            for (i, page) in pages.iter().enumerate() {
                let filename = format!("scan_{timestamp}_page{:02}.jpg", i + 1);
                let path = dir.join(&filename);
                std::fs::write(&path, &page.jpeg_bytes)
                    .with_context(|| format!("write JPEG to {path:?}"))?;
                info!("saved JPEG: {path:?}");
                written.push(path);
            }
        }
    }

    Ok(written)
}

/// Expand `~/` and resolve environment variables in the configured directory.
fn resolve_dir(raw: &str) -> Result<PathBuf> {
    let expanded = if raw.starts_with("~/") {
        let home = dirs::home_dir().context("could not determine home directory")?;
        home.join(&raw[2..])
    } else {
        PathBuf::from(raw)
    };
    Ok(expanded)
}
