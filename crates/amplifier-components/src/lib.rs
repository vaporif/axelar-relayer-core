//! Crate with amplifier component connectors.
use eyre::Context;
use relayer_amplifier_api_integration::Config;
use relayer_amplifier_api_integration::amplifier_api::AmplifierApiClient;

#[cfg(feature = "nats")]
pub mod nats;

#[allow(dead_code)]
fn amplifier_client(config: &Config) -> eyre::Result<AmplifierApiClient> {
    AmplifierApiClient::new(config.url.clone(), &config.identity)
        .wrap_err("amplifier api client failed to create")
}
