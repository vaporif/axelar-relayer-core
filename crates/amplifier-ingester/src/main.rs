//! # Amplifier Ingester
//!
//! The Amplifier Ingester service connects to the Axelar Amplifier network
//! and periodically ingests events from the network that need to be relayed.
//!
//! This binary provides:
//! - A configurable event ingestion service with different backend implementations (NATS, GCP)
//! - Health check endpoints for monitoring service health
//! - Graceful shutdown handling with signal trapping
//!
//! The service will periodically check for new events based on the configured tickrate
//! and process them according to the configured backend (NATS or GCP).
//!
//! ## Usage
//!
//! ```bash
//! amplifier-ingester --config path/to/config.toml
//! ```
//!
//! ## Configuration
//!
//! The service is configured via a TOML file that specifies:
//! - Health check server port
//! - Event processing tickrate
//! - Backend-specific configuration (NATS or GCP)
mod components;
mod config;

use core::time::Duration;

use bin_util::health_check;
use clap::{Parser, crate_name, crate_version};
use config::Config;
use tokio_util::sync::CancellationToken;

#[derive(Parser, Debug)]
#[command(author = "Eiger", name = "Axelar Relayer(amplifier-ingester)")]
pub(crate) struct Cli {
    #[arg(long, short, default_value = "relayer-config", help = "Config path")]
    pub config_path: String,
}

#[tokio::main]
async fn main() {
    bin_util::ensure_backtrace_set();
    let cli = Cli::parse();

    let config: Config = bin_util::try_deserialize(&cli.config_path).expect("config is correct");
    let (telemetry_tracer, _observer_handle) = if let Some(ref telemetry_cfg) = config.telemetry {
        let (tracer, observer_handle) =
            bin_util::telemetry::init(crate_name!(), crate_version!(), telemetry_cfg)
                .expect("telemetry wired up");
        (Some(tracer), Some(observer_handle))
    } else {
        (None, None)
    };

    let _stderr_logging_guard =
        bin_util::init_logging(config.env_filters, config.span_events, telemetry_tracer)
            .expect("logging wired up");

    let cancel_token = bin_util::register_cancel();
    tokio::try_join!(
        spawn_subscriber_worker(
            config.tickrate,
            config.max_errors,
            cli.config_path.clone(),
            &cancel_token
        ),
        spawn_health_check_server(
            config.health_check.port,
            cli.config_path.clone(),
            cancel_token.clone()
        )
    )
    .expect("Failed to join main loop and health server tasks");

    tracing::info!("Amplifier ingester has been shut down");
}

fn spawn_subscriber_worker(
    tickrate: Duration,
    max_errors: u32,
    config_path: String,
    cancel_token: &CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn({
        let cancel_token = cancel_token.clone();
        async move {
            #[cfg(feature = "nats")]
            let ingester = components::nats::new_amplifier_ingester(&config_path)
                .await
                .expect("ingester is created");

            #[cfg(feature = "gcp")]
            let ingester =
                components::gcp::new_amplifier_ingester(&config_path, cancel_token.clone())
                    .await
                    .expect("ingester is created");

            tracing::trace!("Starting amplifier ingester...");

            cancel_token
                .run_until_cancelled(async move {
                    let mut work_interval = tokio::time::interval(tickrate);
                    let mut error_count: u32 = 0;

                    loop {
                        work_interval.tick().await;
                        if let Err(err) = ingester.ingest().await {
                            tracing::error!(?err, "error during ingest, skipping...");
                            error_count = error_count.saturating_add(1);
                            if error_count >= max_errors {
                                tracing::error!("Max error threshold reached. Exiting loop.");
                                break;
                            }
                        } else {
                            error_count = 0_u32;
                        }
                    }
                })
                .await;

            tracing::warn!("Shutting down amplifier ingester...");
        }
    })
}

fn spawn_health_check_server(
    port: u16,
    config_path: String,
    cancel_token: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn(async move {
        tracing::trace!("Starting health check server...");

        let run_token = cancel_token.clone();
        health_check::new(port)
            .add_health_check(move || {
                let config_path = config_path.clone();
                #[allow(unused_variables, reason = "weird bug as cancel token IS USED")]
                let cancel_token = cancel_token.clone();
                async move {
                    #[cfg(feature = "nats")]
                    let ingester = components::nats::new_amplifier_ingester(&config_path)
                        .await
                        .expect("ingester is created");

                    #[cfg(feature = "gcp")]
                    let ingester =
                        components::gcp::new_amplifier_ingester(&config_path, cancel_token)
                            .await
                            .expect("ingester is created");
                    ingester.check_health().await
                }
            })
            .run(run_token)
            .await;

        tracing::warn!("Shutting down health check server...");
    })
}
