pub mod directory;
pub mod email;
pub mod pdf;

use anyhow::Result;
use crate::config::{DestinationConfig, OutputKind};
use crate::scanning::job::ScannedPage;

/// Route completed scan pages to the appropriate output handler.
pub async fn dispatch(config: &DestinationConfig, pages: &[ScannedPage]) -> Result<()> {
    match config.output {
        OutputKind::Directory => {
            directory::save_to_directory(config, pages).await?;
        }
        OutputKind::Email => {
            email::send_scan_email(config, pages).await?;
        }
    }
    Ok(())
}
