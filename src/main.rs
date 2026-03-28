use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{info, warn};

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

    // Outer loop: keeps retrying when the scanner is unreachable or goes
    // offline.  Only exits on a clean shutdown signal.
    loop {
        if *shutdown_rx.borrow() {
            break;
        }

        // Try to register destinations — scanner may be powered off.
        let dest_map = match destinations::register_all(&client, &config).await {
            Ok(map) => map,
            Err(e) => {
                warn!("scanner not reachable: {e} — retrying in 30s");
                if wait_or_shutdown(&mut shutdown_rx.clone(), Duration::from_secs(30)).await {
                    break;
                }
                continue;
            }
        };

        // Run the event loop (returns on error or shutdown).
        let result = app::run(&client, &config, &dest_map, shutdown_rx.clone()).await;

        // Best-effort cleanup.
        destinations::deregister_all(&client, &dest_map).await;

        match result {
            Ok(()) => break, // clean shutdown
            Err(e) => {
                warn!("scanner connection lost: {e} — retrying in 30s");
                if wait_or_shutdown(&mut shutdown_rx.clone(), Duration::from_secs(30)).await {
                    break;
                }
            }
        }
    }

    info!("exiting");
    Ok(())
}

/// Sleep for `duration`, but return early if a shutdown signal arrives.
/// Returns `true` if shutdown was requested.
async fn wait_or_shutdown(
    shutdown: &mut tokio::sync::watch::Receiver<bool>,
    duration: Duration,
) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(duration) => false,
        _ = shutdown.changed() => true,
    }
}
