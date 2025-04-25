use amplifier_api::identity::Identity;
use clap::Parser;
use serde::Deserialize;
use typed_builder::TypedBuilder;

/// global Amplifier component configuration
#[derive(Debug, Deserialize, Clone, PartialEq, TypedBuilder, Parser)]
pub struct Config {
    /// Identity certificate for the Amplifier API authentication to work
    #[arg(env = "AMPLIFIER_API_IDENTITY", value_parser = parse_identity)]
    pub identity: Identity,
    /// The Amplifier API url to connect to
    #[arg(env = "AMPLIFIER_API_URL")]
    pub url: url::Url,
    /// The name of the chain that we need to send / listen for
    #[arg(env = "AMPLIFIER_API_CHAIN")]
    pub chain: String,

    /// The interval between polling Amplifier API for new tasks
    #[builder(default = config_defaults::get_chains_poll_interval())]
    #[serde(
        rename = "get_chains_poll_interval_in_milliseconds",
        default = "config_defaults::get_chains_poll_interval",
        deserialize_with = "common_serde_utils::duration_ms_decode"
    )]
    #[arg(env = "AMPLIFIER_API_CHAINS_POLL_INTERVAL", value_parser = parse_chains_poll_interval)]
    pub get_chains_poll_interval: core::time::Duration,

    /// The max amount of tasks that we want to receive in a batch.
    /// This goes hand-in-hand with the `get_chains_poll_interval`
    #[builder(default = config_defaults::get_chains_limit())]
    #[serde(default = "config_defaults::get_chains_limit")]
    #[arg(env = "AMPLIFIER_API_CHAINS_LIMIT")]
    pub get_chains_limit: u8,

    /// How often we check the liveliness of the Amplifier API
    #[builder(default = config_defaults::healthcheck_interval())]
    #[serde(
        rename = "healthcheck_interval_in_milliseconds",
        default = "config_defaults::healthcheck_interval",
        deserialize_with = "common_serde_utils::duration_ms_decode"
    )]
    #[arg(env = "AMPLIFIER_API_HEALTHCHECK_INTERVAL", value_parser = parse_healthcheck_interval)]
    pub healthcheck_interval: core::time::Duration,

    /// How many invalid healthchecks do we need to do before we deem that the service is down and
    /// we should shut down the component
    #[builder(default = config_defaults::invalid_healthchecks_before_shutdown())]
    #[serde(default = "config_defaults::invalid_healthchecks_before_shutdown")]
    #[arg(env = "AMPLIFIER_API_INVALID_HEALTHCHECKS_BEFORE_SHUTDOWN")]
    pub invalid_healthchecks_before_shutdown: Option<usize>,
}

fn parse_identity(input: &str) -> Result<Identity, String> {
    Ok(Identity::new_from_pem_bytes(input.as_bytes()).expect("failed to parse identity"))
}

fn parse_chains_poll_interval(input: &str) -> Result<core::time::Duration, String> {
    Ok(core::time::Duration::from_secs(
        input
            .parse::<u64>()
            .expect("failed to parse chains poll interval"),
    ))
}

fn parse_healthcheck_interval(input: &str) -> Result<core::time::Duration, String> {
    Ok(core::time::Duration::from_secs(
        input
            .parse::<u64>()
            .expect("failed to parse healthcheck interval"),
    ))
}

pub(crate) mod config_defaults {
    use core::time::Duration;

    pub(crate) const fn healthcheck_interval() -> Duration {
        Duration::from_secs(10)
    }

    pub(crate) const fn get_chains_poll_interval() -> Duration {
        Duration::from_secs(10)
    }

    #[expect(clippy::unnecessary_wraps, reason = "fine for config defaults")]
    pub(crate) const fn invalid_healthchecks_before_shutdown() -> Option<usize> {
        Some(5)
    }
    pub(crate) const fn get_chains_limit() -> u8 {
        4
    }
}
