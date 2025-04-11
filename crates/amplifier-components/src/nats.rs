use eyre::Context as _;
use relayer_amplifier_api_integration::Config;
use relayer_amplifier_api_integration::amplifier_api::{self};
use storage_bus::nats::consumer::NatsConsumer;
use storage_bus::nats::publisher::NatsPublisher;
use storage_bus::nats::{self};
use url::Url;

pub async fn new_amplifier_subscriber(
    config_fn: impl Fn() -> eyre::Result<Config>,
    urls_fn: impl Fn() -> eyre::Result<Vec<Url>>,
) -> eyre::Result<amplifier_subscriber::Subscriber<NatsPublisher<amplifier_api::types::TaskItem>>> {
    let config = config_fn().wrap_err("config file issues")?;
    let nats_urls = urls_fn().wrap_err("nats urls issues")?;
    let amplifier_client = crate::amplifier_client(&config)?;

    let task_queue_publisher = nats::connectors::tasks::connect_publisher(nats_urls)
        .await
        .wrap_err("task queue publisher connect err")?;

    Ok(amplifier_subscriber::Subscriber::new(
        amplifier_client,
        task_queue_publisher,
        config.chain,
    ))
}

pub async fn new_amplifier_ingester(
    config_fn: impl Fn() -> eyre::Result<Config>,
    urls_fn: impl Fn() -> eyre::Result<Vec<Url>>,
) -> eyre::Result<amplifier_ingester::Ingester<NatsConsumer<amplifier_api::types::Event>>> {
    let config = config_fn().wrap_err("config file issues")?;
    let nats_urls = urls_fn().wrap_err("nats urls issues")?;
    let amplifier_client = crate::amplifier_client(&config)?;

    let event_queue_consumer = nats::connectors::events::connect_consumer(nats_urls)
        .await
        .wrap_err("event consumer connect err")?;

    Ok(amplifier_ingester::Ingester::new(
        amplifier_client,
        event_queue_consumer,
        config.chain,
    ))
}
