use std::path::PathBuf;
use std::time::Duration;

use crate::config::{self, Validate, deserialize_duration_from_secs};
use eyre::{Context, ensure, eyre};
use infrastructure::nats::publisher::NatsPublisher;
use infrastructure::nats::{self, StreamArgs};
use relayer_amplifier_api_integration::amplifier_api::{self, AmplifierApiClient};
use serde::Deserialize;
use url::Url;

use crate::Config;

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct NatsSectionConfig {
    pub nats: NatsConfig,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct NatsConfig {
    pub urls: Vec<Url>,
    #[serde(rename = "connection_timeout_secs")]
    #[serde(deserialize_with = "deserialize_duration_from_secs")]
    pub connection_timeout: Duration,

    pub stream_name: String,
    pub stream_subject: String,
    pub stream_description: String,
}

impl Validate for NatsSectionConfig {
    fn validate(&self) -> eyre::Result<()> {
        ensure!(
            !self.nats.urls.is_empty(),
            eyre!("nats urls should have at least one connection")
        );

        Ok(())
    }
}

pub(crate) async fn new_amplifier_subscriber(
    config_path: PathBuf,
) -> eyre::Result<amplifier_subscriber::Subscriber<NatsPublisher<amplifier_api::types::TaskItem>>> {
    let config = config::try_deserialize(&config_path).wrap_err("config file issues")?;
    let nats_config: NatsSectionConfig =
        config::try_deserialize(&config_path).wrap_err("nats config issues")?;
    let amplifier_client = amplifier_client(&config)?;

    let stream = StreamArgs {
        name: nats_config.nats.stream_name.to_owned(),
        subject: nats_config.nats.stream_subject.to_owned(),
        description: nats_config.nats.stream_description.to_owned(),
    };

    let task_queue_publisher = nats::connectors::connect_publisher(
        &nats_config.nats.urls,
        stream,
        nats_config.nats.stream_subject,
    )
    .await
    .wrap_err("task queue publisher connect err")?;

    Ok(amplifier_subscriber::Subscriber::new(
        amplifier_client,
        task_queue_publisher,
        config.amplifier_component.chain.clone(),
    ))
}

fn amplifier_client(config: &Config) -> eyre::Result<AmplifierApiClient> {
    AmplifierApiClient::new(
        config.amplifier_component.url.clone(),
        amplifier_api::TlsType::Certificate(Box::new(config.amplifier_component.identity.clone())),
    )
    .wrap_err("amplifier api client failed to create")
}
