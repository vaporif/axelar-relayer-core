use bin_util::ValidateConfig;
use eyre::{Context as _, ensure, eyre};
use infrastructure::nats::consumer::NatsConsumer;
use infrastructure::nats::{self, StreamArgs};
use relayer_amplifier_api_integration::amplifier_api::{self, AmplifierApiClient};
use serde::Deserialize;
use url::Url;

use crate::Ingester;
use crate::config::Config;

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct NatsSectionConfig {
    pub nats: NatsConfig,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct NatsConfig {
    pub urls: Vec<Url>,

    pub stream_name: String,
    pub stream_subject: String,
    pub stream_description: String,

    pub consumer_description: String,
    pub deliver_group: String,
}

impl ValidateConfig for NatsSectionConfig {
    fn validate(&self) -> eyre::Result<()> {
        ensure!(
            !self.nats.urls.is_empty(),
            eyre!("nats urls should have at least one connection")
        );

        Ok(())
    }
}

/// Creates a new Amplifier ingester configured for NATS messaging.
///
/// This function initializes an ingester that consumes events from a NATS stream
/// and forwards them to the Amplifier API. It sets up the necessary NATS consumer
/// connection and configures the Amplifier API client with TLS authentication.
///
/// # Arguments
///
/// * `config_path` - Path to the configuration file containing both general ingester settings and
///   NATS-specific configuration
///
/// # Returns
///
/// Returns an `Ingester` instance configured with a NATS consumer for processing
/// Amplifier API events, or an error if initialization fails.
///
/// # Configuration
///
/// The configuration file must contain:
/// - General ingester configuration (`concurrent_queue_items`, `amplifier_component`)
/// - NATS configuration section with:
///   - `urls`: List of NATS server URLs
///   - `stream_name`: Name of the NATS stream
///   - `stream_subject`: Subject pattern for the stream
///   - `stream_description`: Description of the stream
///   - `consumer_description`: Description for the consumer
///   - `deliver_group`: Delivery group name for load balancing
///
/// # Errors
///
/// This function will return an error if:
/// - Configuration file cannot be read or parsed
/// - NATS connection cannot be established
/// - Amplifier API client fails to initialize
/// - Required configuration fields are missing
pub async fn new_amplifier_ingester(
    config_path: &str,
) -> eyre::Result<Ingester<NatsConsumer<amplifier_api::types::Event>>> {
    let config: Config = bin_util::try_deserialize(config_path)?;
    let nats_config: NatsSectionConfig = bin_util::try_deserialize(config_path)?;

    let amplifier_client = amplifier_client(&config)?;

    let stream = StreamArgs {
        name: nats_config.nats.stream_name.clone(),
        subject: nats_config.nats.stream_subject.clone(),
        description: nats_config.nats.stream_description.clone(),
    };

    let event_queue_consumer = nats::connectors::connect_consumer(
        &nats_config.nats.urls,
        stream,
        nats_config.nats.consumer_description,
        nats_config.nats.deliver_group,
    )
    .await
    .wrap_err("event consumer connect err")?;

    Ok(Ingester::new(
        amplifier_client,
        event_queue_consumer,
        config.concurrent_queue_items,
        config.amplifier_component.chain.clone(),
    ))
}

fn amplifier_client(config: &Config) -> eyre::Result<AmplifierApiClient> {
    AmplifierApiClient::new(
        config.amplifier_component.url.clone(),
        amplifier_api::TlsType::Certificate(Box::new(
            config
                .amplifier_component
                .identity
                .clone()
                .ok_or_else(|| eyre::Report::msg("identity not set"))?,
        )),
    )
    .wrap_err("amplifier api client failed to create")
}
