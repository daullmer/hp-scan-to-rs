use anyhow::{bail, Context, Result};
use base64::Engine;
use chrono::Local;
use serde::Serialize;
use tracing::info;

use crate::config::DestinationConfig;
use crate::output::pdf::pages_to_pdf;
use crate::scanning::job::ScannedPage;

const RESEND_API_URL: &str = "https://api.resend.com/emails";

#[derive(Serialize)]
struct EmailAttachment {
    filename: String,
    content: String, // base64-encoded
}

#[derive(Serialize)]
struct SendEmailRequest {
    from: String,
    to: Vec<String>,
    subject: String,
    html: String,
    attachments: Vec<EmailAttachment>,
}

/// Assemble pages into a PDF and send it as an email attachment via Resend.
pub async fn send_scan_email(config: &DestinationConfig, pages: &[ScannedPage]) -> Result<()> {
    let api_key = std::env::var("RESEND_API_KEY")
        .context("RESEND_API_KEY environment variable is not set")?;

    let to_addr = config.to.as_deref().context("email destination missing `to`")?;
    let from_addr = config.from.as_deref().context("email destination missing `from`")?;
    let subject = config
        .subject
        .as_deref()
        .unwrap_or("Scan from HP");

    info!("assembling PDF for email to {to_addr}");
    let pdf_bytes = pages_to_pdf(pages)?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let filename = format!("scan_{timestamp}.pdf");

    let content = base64::engine::general_purpose::STANDARD.encode(&pdf_bytes);

    let body = SendEmailRequest {
        from: from_addr.to_string(),
        to: vec![to_addr.to_string()],
        subject: subject.to_string(),
        html: format!("<p>Please find your scan attached ({} page(s)).</p>", pages.len()),
        attachments: vec![EmailAttachment { filename, content }],
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(RESEND_API_URL)
        .bearer_auth(&api_key)
        .json(&body)
        .send()
        .await
        .context("failed to send request to Resend API")?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        bail!("Resend API returned {status}: {text}");
    }

    info!("email sent to {to_addr} via Resend");
    Ok(())
}
