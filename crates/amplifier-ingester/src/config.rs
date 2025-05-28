use core::time::Duration;

use bin_util::config_defaults::{default_concurrent_queue_items, default_max_errors};
use bin_util::telemetry::FmtSpan;
use bin_util::{ValidateConfig, deserialize_duration_from_secs, deserialize_fmt_span_option};
use eyre::ensure;
use serde::Deserialize;

/// Top-level configuration for the relayer.
#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    /// Per-crate log levels (e.g. `my_crate` = "debug")
    pub env_filters: Option<Vec<String>>,
    /// Configures how synthesized events are emitted at points in the [span
    /// lifecycle][lifecycle].
    #[serde(deserialize_with = "deserialize_fmt_span_option")]
    pub span_events: Option<FmtSpan>,
    /// Configuration for the Amplifier API processor
    pub amplifier_component: relayer_amplifier_api_integration::Config,
    /// Duration (in seconds) to wait between consecutive polling
    /// operations Used to prevent overwhelming the network with requests
    #[serde(rename = "tickrate_secs")]
    #[serde(deserialize_with = "deserialize_duration_from_secs")]
    pub tickrate: Duration,
    pub telemetry: Option<bin_util::telemetry::Config>,
    #[serde(rename = "health_check_server")]
    pub health_check: bin_util::health_check::Config,
    /// Maximum consecutive errors allowed before application termination.
    #[serde(default = "default_max_errors")]
    pub max_errors: u32,
    #[serde(default = "default_concurrent_queue_items")]
    pub concurrent_queue_items: usize,
}

impl ValidateConfig for Config {
    fn validate(&self) -> eyre::Result<()> {
        ensure!(
            !self.amplifier_component.chain.trim().is_empty(),
            "chain could not be empty",
        );

        Ok(())
    }
}
