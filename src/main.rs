use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tracing::info;

mod app;
mod config;
mod destinations;
mod output;
mod scanner;
mod scanning;

use config::{default_config_path, Config};
use scanner::client::ScannerClient;

#[derive(Parser, Debug)]
#[command(
    name = "hp-scan-to",
    about = "Scan to directory or email from HP network scanners"
)]
struct Cli {
    /// Path to the TOML configuration file.
    /// Defaults to ~/.config/hp-scan-to/config.toml
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Override the scanner IP from the config file.
    #[arg(long, value_name = "IP")]
    ip: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialise structured logging.  Level can be controlled via RUST_LOG.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hp_scan_to=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    let config_path = cli.config.unwrap_or_else(default_config_path);

    let mut config = Config::load(&config_path)
        .with_context(|| format!("failed to load config from {config_path:?}"))?;

    // CLI --ip overrides the config file.
    if let Some(ip) = cli.ip {
        config.scanner.ip = ip;
    }

    info!("scanner IP: {}", config.scanner.ip);
    info!(
        "destinations: {}",
        config
            .destinations
            .iter()
            .map(|d| d.label.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let client = ScannerClient::new(&config.scanner.ip)?;

    // Register destinations with the scanner.
    let dest_map = destinations::register_all(&client, &config).await?;

    // Set up a shutdown channel driven by SIGINT / SIGTERM.
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    tokio::spawn(async move {
        let ctrl_c = tokio::signal::ctrl_c();
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate())
                .expect("failed to install SIGTERM handler");
            tokio::select! {
                _ = ctrl_c => {},
                _ = sigterm.recv() => {},
            }
        }
        #[cfg(not(unix))]
        {
            let _ = ctrl_c.await;
        }
        info!("shutdown signal received");
        let _ = shutdown_tx.send(true);
    });

    // Run the event loop.
    let result = app::run(&client, &config, &dest_map, shutdown_rx).await;

    // Always clean up destinations before exiting.
    destinations::deregister_all(&client, &dest_map).await;

    result
}
