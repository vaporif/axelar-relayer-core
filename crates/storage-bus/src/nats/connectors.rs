pub mod events {
    use url::Url;

    use crate::nats::NatsBuilder;
    use crate::nats::consumer::NatsConsumer;
    use crate::nats::error::Error;
    use crate::nats::publisher::NatsPublisher;

    const AMPLIFIER_EVENTS_STREAM: &str = "AMPLIFIER_EVENTS";
    const AMPLIFIER_EVENTS_PUBLISH_SUBJECT: &str = "amplifier.event.new";

    pub async fn connect_consumer(
        urls: Vec<Url>,
    ) -> Result<NatsConsumer<amplifier_api::types::Event>, Error> {
        let consumer = NatsBuilder::connect_to_nats(urls)
            .await?
            .stream(
                AMPLIFIER_EVENTS_STREAM,
                AMPLIFIER_EVENTS_PUBLISH_SUBJECT,
                "amplifier events to send to amplifier api",
            )
            .await?
            .consumer("amplifier events consumer", "permissionless-consumers")
            .await?;
        Ok(consumer)
    }

    pub async fn connect_publisher(
        urls: Vec<Url>,
    ) -> Result<NatsPublisher<amplifier_api::types::Event>, Error> {
        let publisher = NatsBuilder::connect_to_nats(urls)
            .await?
            .stream(
                AMPLIFIER_EVENTS_STREAM,
                AMPLIFIER_EVENTS_PUBLISH_SUBJECT,
                "amplifier events to send to amplifier api",
            )
            .await?
            .publisher(AMPLIFIER_EVENTS_PUBLISH_SUBJECT)?;
        Ok(publisher)
    }
}

pub mod tasks {
    use url::Url;

    use crate::nats::NatsBuilder;
    use crate::nats::consumer::NatsConsumer;
    use crate::nats::error::Error;
    use crate::nats::publisher::NatsPublisher;

    const AMPLIFIER_TASKS_STREAM: &str = "AMPLIFIER_TASKS";
    const AMPLIFIER_TASKS_PUBLISH_SUBJECT: &str = "amplifier.tasks.new";

    pub async fn connect_consumer(
        urls: Vec<Url>,
    ) -> Result<NatsConsumer<amplifier_api::types::TaskItem>, Error> {
        let consumer = NatsBuilder::connect_to_nats(urls)
            .await?
            .stream(
                AMPLIFIER_TASKS_STREAM,
                AMPLIFIER_TASKS_PUBLISH_SUBJECT,
                "amplifier tasks for ingester in starknet",
            )
            .await?
            .consumer("amplifier tasks consumer", "permissionless-consumers")
            .await?;
        Ok(consumer)
    }

    pub async fn connect_publisher(
        urls: Vec<Url>,
    ) -> Result<NatsPublisher<amplifier_api::types::TaskItem>, Error> {
        let publisher = NatsBuilder::connect_to_nats(urls)
            .await?
            .stream(
                AMPLIFIER_TASKS_STREAM,
                AMPLIFIER_TASKS_PUBLISH_SUBJECT,
                "amplifier tasks for ingester in starknet",
            )
            .await?
            .publisher(AMPLIFIER_TASKS_PUBLISH_SUBJECT)?;
        Ok(publisher)
    }
}
