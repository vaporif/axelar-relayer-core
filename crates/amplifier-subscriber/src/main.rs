//! # Amplifier Subscriber
//!
//! The Amplifier Subscriber service connects to the Axelar Amplifier network
//! and subscribes to events that need to be relayed.
//!
//! This binary provides:
//! - A configurable event subscriber with different backend implementations (NATS, GCP)
//! - Health check endpoints for monitoring service health
//! - Graceful shutdown handling with signal trapping
//!
//! The service will periodically poll for events based on the configured tickrate
//! and process them according to the configured backend (NATS or GCP Pub/Sub).
//!
//! ## Usage
//!
//! ```bash
//! amplifier-subscriber --config path/to/config.toml
//! ```
//!
//! ## Configuration
//!
//! The service is configured via a TOML file that specifies:
//! - Health check server port
//! - Event processing tickrate
//! - Backend-specific configuration (NATS or GCP)
//!
mod components;
mod config;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use config::Config;
use tokio_util::sync::CancellationToken;

#[derive(Parser, Debug)]
#[command(author = "Eiger", name = "Axelar<>Blockchain Relayer")]
pub(crate) struct Cli {
    #[arg(
        long,
        short,
        default_value = "relayer-config.toml",
        help = "Config path"
    )]
    pub config_path: PathBuf,
}

#[tokio::main]
async fn main() {
    #[cfg(debug_assertions)]
    {
        // Note: unsafe only in multithreaded
        unsafe {
            std::env::set_var("RUST_BACKTRACE", "full");
        }
    }
    let cli = Cli::parse();

    let config: Config = config::try_deserialize(&cli.config_path).expect("generic config");
    let cancel_token = setup_shutdown_signal();

    tokio::try_join!(
        spawn_subscriber_worker(
            config.tickrate,
            cli.config_path.clone(),
            cancel_token.clone()
        ),
        spawn_health_check_server(
            config.health_check.port,
            cli.config_path.clone(),
            cancel_token.clone()
        )
    )
    .expect("Failed to join tasks");

    tracing::info!("Amplifier subscriber has been shut down");
}

fn setup_shutdown_signal() -> CancellationToken {
    let token = CancellationToken::new();
    let token_clone = token.clone();

    ctrlc::set_handler(move || {
        if token_clone.is_cancelled() {
            std::process::exit(1);
        } else {
            println!("\nGraceful shutdown initiated. Press Ctrl+C again for immediate exit...");
            token_clone.cancel();
        }
    })
    .expect("Failed to register ctrl+c handler");

    token
}

fn spawn_subscriber_worker(
    tickrate: Duration,
    config_path: PathBuf,
    cancel_token: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    let cancel_token = cancel_token.clone();
    tokio::task::spawn(async move {
        #[cfg(feature = "nats")]
        let mut subscriber = components::nats::new_amplifier_subscriber(config_path)
            .await
            .expect("subscriber is created");

        #[cfg(feature = "gcp")]
        let mut subscriber = components::gcp::new_amplifier_subscriber(config_path)
            .await
            .expect("subscriber is created");

        tracing::debug!("Starting amplifier subscriber...");

        cancel_token
            .run_until_cancelled(async move {
                let mut work_interval = tokio::time::interval(tickrate);
                loop {
                    work_interval.tick().await;
                    if let Err(err) = subscriber.subscribe().await {
                        tracing::error!(?err, "error during ingest, skipping...");
                        // TODO: health check should fail at threshold
                    }
                }
            })
            .await;

        tracing::warn!("Shutting down amplifier subscriber...");
    })
}

fn spawn_health_check_server(
    port: u16,
    config_path: PathBuf,
    cancel_token: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn(async move {
        #[cfg(feature = "nats")]
        let subscriber = components::nats::new_amplifier_subscriber(config_path)
            .await
            .expect("subscriber is created");

        #[cfg(feature = "gcp")]
        let subscriber = components::gcp::new_amplifier_subscriber(config_path)
            .await
            .expect("subscriber is created");

        let subscriber = Arc::new(subscriber);

        tracing::debug!("Starting health check server...");

        health_check::new(port)
            .add_health_check(move || {
                let subscriber = Arc::clone(&subscriber);
                async move { subscriber.check_health().await }
            })
            .run(cancel_token)
            .await;

        tracing::warn!("Shutting down health check server...");
    })
}
