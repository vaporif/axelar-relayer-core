use eyre::Context;
use relayer_amplifier_api_integration::Config;
use relayer_amplifier_api_integration::amplifier_api::{self, AmplifierApiClient};
use storage_bus::nats::consumer::NatsConsumer;
use storage_bus::nats::kv_store::NatsKvStore;
use storage_bus::nats::publisher::NatsPublisher;
use storage_bus::nats::{self};
use url::Url;

pub async fn new_amplifier_subscriber(
    config_fn: impl Fn() -> eyre::Result<Config>,
    urls_fn: impl Fn() -> Vec<Url>,
    chain_name: String,
) -> eyre::Result<amplifier_subscriber::Subscriber<NatsPublisher<amplifier_api::types::TaskItem>>> {
    let config = config_fn().wrap_err("config file issues")?;
    let amplifier_client = crate::amplifier_client(&config)?;

    let task_queue_publisher = nats::connectors::tasks::connect_publisher(urls_fn())
        .await
        .wrap_err("task queue publisher connect err")?;

    Ok(amplifier_subscriber::Subscriber::new(
        amplifier_client,
        task_queue_publisher,
        chain_name,
    ))
}

pub async fn new_amplifier_ingester(
    config_fn: impl Fn() -> eyre::Result<Config>,
    urls_fn: impl Fn() -> Vec<Url>,
    chain_name: String,
) -> eyre::Result<amplifier_ingester::Ingester<NatsConsumer<amplifier_api::types::Event>>> {
    let config = config_fn().wrap_err("config file issues")?;
    let amplifier_client = crate::amplifier_client(&config)?;

    let event_queue_consumer = nats::connectors::events::connect_consumer(urls_fn())
        .await
        .wrap_err("event consumer connect err")?;

    Ok(amplifier_ingester::Ingester::new(
        amplifier_client,
        event_queue_consumer,
        chain_name,
    ))
}
