use anyhow::{Context, Result};
use reqwest::{Client, Response, StatusCode};
use std::time::Duration;
use tracing::{debug, trace};

/// Low-level HTTP client for the HP scanner.
/// All methods are thin wrappers around `reqwest` that add the base URL and
/// common headers.
#[derive(Clone)]
pub struct ScannerClient {
    client: Client,
    base: String,
}

impl ScannerClient {
    pub fn new(ip: &str) -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(300))
            .no_proxy()
            .http1_title_case_headers()
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self {
            client,
            base: format!("http://{}", ip),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base
    }

    pub async fn get(&self, path: &str) -> Result<Response> {
        let url = format!("{}{}", self.base, path);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;
        Ok(resp)
    }

    /// GET with an If-None-Match header for ETag-based conditional polling.
    pub async fn get_conditional(
        &self,
        path: &str,
        etag: Option<&str>,
    ) -> Result<Response> {
        let url = format!("{}{}", self.base, path);
        let mut req = self.client.get(&url);
        if let Some(tag) = etag {
            req = req.header("If-None-Match", tag);
        }
        let resp = req
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;
        Ok(resp)
    }

    pub async fn delete(&self, path: &str) -> Result<StatusCode> {
        let url = format!("{}{}", self.base, path);
        let resp = self
            .client
            .delete(&url)
            .send()
            .await
            .with_context(|| format!("DELETE {url}"))?;
        Ok(resp.status())
    }

    /// POST XML to a full absolute URL.
    pub async fn post_xml_url(&self, url: &str, body: String) -> Result<Response> {
        trace!("POST {url}\n{body}");
        let resp = self
            .client
            .post(url)
            .header("Content-Type", "text/xml")
            .body(body)
            .send()
            .await
            .with_context(|| format!("POST {url}"))?;
        debug!("POST {url} => {}", resp.status());
        Ok(resp)
    }
}
