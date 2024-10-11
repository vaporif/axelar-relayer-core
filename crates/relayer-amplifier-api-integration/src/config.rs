use amplifier_api::identity::Identity;
use serde::Deserialize;
use typed_builder::TypedBuilder;

/// global Amplifier component configuration
#[derive(Debug, Deserialize, Clone, PartialEq, TypedBuilder)]
pub struct Config {
    /// Identity certificate for the Amplifier API authentication to work
    pub identity: Identity,
    /// The Amplifier API url to connect to
    pub url: url::Url,
    /// The name of the chain that we need to send / listen for
    pub chain: String,

    /// The interval between polling Amplifier API for new tasks
    #[builder(default = config_defaults::get_chains_poll_interval())]
    #[serde(
        rename = "get_chains_poll_interval_in_milliseconds",
        default = "config_defaults::get_chains_poll_interval",
        deserialize_with = "common_serde_utils::duration_ms_decode"
    )]
    pub get_chains_poll_interval: core::time::Duration,

    /// The max amount of tasks that we want to receive in a batch.
    /// This goes hand-in-hand with the `get_chains_poll_interval`
    #[builder(default = config_defaults::get_chains_limit())]
    #[serde(default = "config_defaults::get_chains_limit")]
    pub get_chains_limit: u8,

    /// How often we check the liveliness of the Amplifier API
    #[builder(default = config_defaults::healthcheck_interval())]
    #[serde(
        rename = "healthcheck_interval_in_milliseconds",
        default = "config_defaults::healthcheck_interval",
        deserialize_with = "common_serde_utils::duration_ms_decode"
    )]
    pub healthcheck_interval: core::time::Duration,

    /// How many invalid healthchecks do we need to do before we deem that the service is down and
    /// we should shut down the component
    #[builder(default = config_defaults::invalid_healthchecks_before_shutdown())]
    #[serde(default = "config_defaults::invalid_healthchecks_before_shutdown")]
    pub invalid_healthchecks_before_shutdown: Option<usize>,
}

pub(crate) mod config_defaults {
    use core::time::Duration;

    pub(crate) const fn healthcheck_interval() -> Duration {
        Duration::from_secs(10)
    }

    pub(crate) const fn get_chains_poll_interval() -> Duration {
        Duration::from_secs(3)
    }

    #[expect(clippy::unnecessary_wraps, reason = "fine for config defaults")]
    pub(crate) const fn invalid_healthchecks_before_shutdown() -> Option<usize> {
        Some(5)
    }
    pub(crate) const fn get_chains_limit() -> u8 {
        25
    }
}
