use anyhow::{bail, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub scanner: ScannerConfig,
    pub destinations: Vec<DestinationConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ScannerConfig {
    pub ip: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DestinationConfig {
    /// Unique label shown on the scanner LCD.
    pub label: String,
    pub output: OutputKind,

    // ── directory output ──────────────────────────────────────────────────
    /// Required when output = "directory"
    pub directory: Option<String>,
    /// "pdf" or "jpeg". Required when output = "directory".
    pub format: Option<FileFormat>,

    // ── email output ──────────────────────────────────────────────────────
    /// Required when output = "email"
    pub to: Option<String>,
    /// Required when output = "email"
    pub from: Option<String>,
    /// Optional — defaults to "Scan from HP"
    pub subject: Option<String>,

    // ── scan settings ─────────────────────────────────────────────────────
    /// DPI — default 200
    #[serde(default = "default_resolution")]
    pub resolution: u32,
    /// "color", "gray", or "bw" — default "color"
    #[serde(default = "default_color_mode")]
    pub color_mode: ColorMode,
    /// "a4", "letter", "legal", "a5", "b5", "max" — default "a4"
    #[serde(default = "default_paper_size")]
    pub paper_size: PaperSize,
    /// Enable duplex (double-sided) scanning — default false
    #[serde(default)]
    pub duplex: bool,
    /// Optional directory for the unmodified JPEG responses from the scanner.
    pub raw_jpeg_directory: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputKind {
    Directory,
    Email,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FileFormat {
    Pdf,
    Jpeg,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode {
    Color,
    Gray,
    Bw,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PaperSize {
    A3,
    A4,
    A5,
    B5,
    Letter,
    Legal,
    Max,
}

fn default_resolution() -> u32 {
    200
}

fn default_color_mode() -> ColorMode {
    ColorMode::Color
}

fn default_paper_size() -> PaperSize {
    PaperSize::A4
}

impl Config {
    pub fn load(path: &PathBuf) -> Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&text)?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.destinations.is_empty() {
            bail!("config must define at least one [[destinations]] entry");
        }

        // Labels must be unique.
        let mut seen = std::collections::HashSet::new();
        for d in &self.destinations {
            if d.label.is_empty() {
                bail!("destination label must not be empty");
            }
            if !seen.insert(d.label.clone()) {
                bail!("duplicate destination label: {:?}", d.label);
            }
        }

        for d in &self.destinations {
            match d.output {
                OutputKind::Directory => {
                    if d.directory.is_none() {
                        bail!(
                            "destination {:?}: output = \"directory\" requires a `directory` field",
                            d.label
                        );
                    }
                    if d.format.is_none() {
                        bail!(
                            "destination {:?}: output = \"directory\" requires a `format` field (\"pdf\" or \"jpeg\")",
                            d.label
                        );
                    }
                }
                OutputKind::Email => {
                    if d.to.is_none() {
                        bail!(
                            "destination {:?}: output = \"email\" requires a `to` field",
                            d.label
                        );
                    }
                    if d.from.is_none() {
                        bail!(
                            "destination {:?}: output = \"email\" requires a `from` field",
                            d.label
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

/// Default config file path: ~/.config/hp-scan-to/config.toml
pub fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hp-scan-to")
        .join("config.toml")
}
